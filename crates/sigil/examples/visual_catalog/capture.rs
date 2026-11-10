use sigil::Error;
use sigil::Host;
use sigil::events::{Event, MouseButton, MouseButtons, PointerPhase};
use sigil::geometry::Point;
use sigil::prelude::*;
use sigil::reactive::Owner;
use sigil::style::Token;
use sigil::terminal::{Cell, Grid, Ink};
use sigil_kittest::{Harness, InteractionMode};

use std::time::Duration;

use sigil::tasks::AsyncRuntime;

use super::catalog::Settle;
use super::spec::State;
use super::tokens::hex;

const DT: f32 = 1.0 / 60.0;

#[derive(Clone, Copy)]
pub(crate) enum Look {
    Light,
    Dark,
}

impl Look {
    pub(crate) const ALL: [Self; 2] = [Self::Light, Self::Dark];

    pub(crate) const fn tag(self) -> &'static str {
        match self {
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }

    pub(crate) fn theme(self) -> Theme {
        match self {
            Self::Light => Theme::default(),
            Self::Dark => Theme::dark(),
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct View {
    pub(crate) mode: InteractionMode,
    pub(crate) size: [u32; 2],
    pub(crate) pad: f32,
    pub(crate) hit: (f32, f32),
}

pub(crate) fn paint<N: IntoNode + 'static>(
    child: impl FnOnce() -> N,
    (look, view, scale): (Look, View, f32),
    mut settle: Option<(Settle, &mut AsyncRuntime)>,
) -> super::shots::Capture {
    let mut left = settle.as_ref().map_or(0, |(settle, _)| settle.frames);
    let mut wait = || {
        let Some((settle, tasks)) = settle.as_mut().filter(|_| left > 0) else {
            return false;
        };
        left -= 1;
        std::thread::sleep(Duration::from_millis(settle.ms));
        tasks.pump();
        true
    };
    let theme = look.theme();
    let pad = view.pad;
    let frame = move || {
        column()
            .theme(theme)
            .background(Token::Surface)
            .fill()
            .padding(Space::raw(pad, "example layout"))
            .gap(Space::None)
            .child(child())
    };
    let extra = (2.0 * pad) as u32;
    Harness::render(frame)
        .focus_first()
        .mode(view.mode)
        .size([view.size[0] + extra, view.size[1] + extra])
        .scale(scale)
        .hit((view.hit.0 + pad, view.hit.1 + pad))
        .capture(&mut wait)
}

pub(crate) struct Cells {
    pub(crate) grid: Grid,
    pub(crate) rows: usize,
}

pub(crate) struct Run<'a> {
    pub(crate) look: Look,
    pub(crate) events: Vec<Event>,
    pub(crate) focus: bool,
    pub(crate) cells: (u16, u16),
    pub(crate) settle: Settle,
    pub(crate) tasks: &'a mut AsyncRuntime,
}

pub(crate) fn grid(tree: impl IntoNode + 'static, run: Run<'_>) -> Result<Cells, Error> {
    let (cols, rows) = run.cells;
    let tasks = run.tasks;
    let settle = run.settle;
    Owner::scope("visual-catalog-terminal", || {
        let mut host = Host::new(HostMode::Cell);
        host.mount(move || {
            column()
                .theme(run.look.theme())
                .fill()
                .gap(Space::None)
                .child(tree)
        });
        host.layout((f32::from(cols) * Host::CELL, f32::from(rows) * Host::CELL))?;
        if run.focus {
            host.repair_focus();
            let (cx, layout, router) = host.parts();
            router.focus_next(cx, layout);
        }
        let mut grid = host.grid(run.events, cols, rows)?;
        for _ in 0..SETTLE_FRAMES + settle.frames {
            std::thread::sleep(Duration::from_millis(settle.ms));
            tasks.pump();
            grid = host.grid([], cols, rows)?;
        }
        Ok(Cells {
            grid,
            rows: usize::from(rows),
        })
    })
}

const SETTLE_FRAMES: u32 = 3;

pub(crate) fn drive(state: State, hit: (f32, f32), drag: [f32; 2]) -> Vec<Event> {
    let phases: &[PointerPhase] = match state {
        State::Hover => &[PointerPhase::Move],
        State::Pressed | State::Dragged => &[PointerPhase::Move, PointerPhase::Down],
        State::Open => &[PointerPhase::Move, PointerPhase::Down, PointerPhase::Up],
        _ => &[],
    };
    let mut events: Vec<Event> = phases
        .iter()
        .map(|phase| pointer(hit, *phase, *phase == PointerPhase::Down))
        .collect();
    if state == State::Dragged {
        let target = (hit.0 + drag[0], hit.1 + drag[1]);
        events.push(pointer(target, PointerPhase::Move, true));
    }
    events
}

fn pointer(hit: (f32, f32), phase: PointerPhase, pressed: bool) -> Event {
    Event::pointer(phase, Point::new(hit.0, hit.1))
        .button((phase != PointerPhase::Move).then_some(MouseButton::Left))
        .buttons(if pressed {
            MouseButtons::LEFT
        } else {
            MouseButtons::empty()
        })
}

pub(crate) struct Term {
    pub(crate) html: String,
    pub(crate) cols: usize,
    pub(crate) rows: usize,
}

pub(crate) fn term(cells: &Cells, look: Look) -> Term {
    let theme = look.theme();
    let grid = &cells.grid;
    let rows = (0..cells.rows)
        .rev()
        .find(|row| grid.painted_width(*row) > 0)
        .map_or(0, |last| last + 1);
    let cols = (0..rows)
        .map(|row| grid.painted_width(row))
        .max()
        .unwrap_or(0);
    let mut out = format!(
        "<pre class=\"term\" style=\"color:{};background:{}\">",
        hex(theme.color(Token::OnSurface)),
        hex(theme.color(Token::Surface))
    );
    for row in 0..rows {
        for cell in grid.row(row).iter().take(cols) {
            if cell.text().is_empty() {
                continue;
            }
            let style = cell_style(cell, &theme);
            let text = escape(cell.text());
            if style.is_empty() {
                out.push_str(&text);
            } else {
                out.extend(["<span style=\"", &style, "\">", &text, "</span>"]);
            }
        }
        out.push('\n');
    }
    out.push_str("</pre>");
    Term {
        html: out,
        cols,
        rows,
    }
}

fn cell_style(cell: &Cell, theme: &Theme) -> String {
    let mut style = String::new();
    if let Some(colour) = ink_css(cell.fg, theme) {
        style.extend(["color:", &colour, ";"]);
    }
    if let Some(colour) = ink_css(cell.bg, theme) {
        style.extend(["background:", &colour, ";"]);
    }
    if cell.bold {
        style.push_str("font-weight:bold;");
    }
    if cell.italic {
        style.push_str("font-style:italic;");
    }
    style
}

fn ink_css(ink: Ink, theme: &Theme) -> Option<String> {
    match ink {
        Ink::Exact(rgba) => Some(hex(rgba)),
        Ink::Of(tone) => Token::scale(tone.scale_role(), 11).map(|token| hex(theme.color(token))),
        _ => None,
    }
}

pub(crate) fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

pub(crate) const fn mode(state: State, drag: [f32; 2]) -> InteractionMode {
    match state {
        State::Dragged => InteractionMode::Dragged {
            dx: drag[0],
            dy: drag[1],
        },
        State::Hover => InteractionMode::HoverAfterFrames { frames: 60, dt: DT },
        State::Pressed => InteractionMode::PressedAfterFrames { frames: 60, dt: DT },
        State::Focused => InteractionMode::FocusedAfterFrames { frames: 60, dt: DT },
        State::Open => InteractionMode::Clicked,
        State::Default | State::Disabled | State::Shown | State::Invalid | State::Selected => {
            InteractionMode::Default
        }
    }
}
