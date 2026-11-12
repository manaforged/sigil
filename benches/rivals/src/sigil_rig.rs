use std::hint::black_box;

use sigil::Host;
use sigil::events::Event;
use sigil::geometry::Point;
use sigil::prelude::*;
use sigil::reactive::Owner;
use sigil::terminal::Screen;

use crate::measure::{Change, Rig};
use crate::scene::{BODY, COLS, FRAME_SECONDS, LINES, MESSAGES, extend, title, view};

const ESTIMATE_ROWS: f32 = 2.0;

/// What one Sigil frame produces.
#[derive(Clone, Copy)]
pub enum Surface {
    Scene,
    Grid,
    Screen,
}

/// How the chat is built.
#[derive(Clone, Copy)]
pub enum Tree {
    Scroll,
    Virtual,
}

pub struct SigilRig {
    host: Host,
    surface: Surface,
    stream: Signal<String>,
    narrow: bool,
    screen: Option<Screen>,
    out: Vec<u8>,
}

fn message(index: usize) -> NodeBuilder {
    column()
        .child(text(title(index)).nowrap())
        .child(text(BODY))
}

fn chat(stream: Signal<String>) -> impl IntoNode {
    let history = (0..MESSAGES).fold(column().gap(Space::Sm), |page, index| {
        page.child(message(index))
    });
    scroll(history.child(text(move || stream.get()))).fill()
}

fn windowed(stream: Signal<String>) -> impl IntoNode {
    let items: Vec<usize> = (0..=MESSAGES).collect();
    virtual_list(items, move |_, index: &usize| {
        if *index < MESSAGES {
            message(*index)
        } else {
            column().child(text(move || stream.get()))
        }
    })
    .measured()
    .estimate(rows(ESTIMATE_ROWS))
    .fill()
}

impl SigilRig {
    pub fn new(surface: Surface, tree: Tree) -> Self {
        let stream = signal(String::new());
        let mode = match surface {
            Surface::Scene => HostMode::Pixel,
            Surface::Grid | Surface::Screen => HostMode::Cell,
        };
        let mut host = Host::new(mode);
        match tree {
            Tree::Scroll => host.mount(chat(stream)),
            Tree::Virtual => host.mount(windowed(stream)),
        }
        Self {
            host,
            surface,
            stream,
            narrow: false,
            screen: matches!(surface, Surface::Screen).then(|| Screen::new(COLS, LINES)),
            out: Vec::new(),
        }
    }

    fn cells(&self) -> (u16, u16) {
        (COLS - u16::from(self.narrow), LINES)
    }

    fn view(&self) -> (f32, f32) {
        match self.surface {
            Surface::Scene => view(self.narrow),
            Surface::Grid | Surface::Screen => {
                let (cols, lines) = self.cells();
                (f32::from(cols) * Host::CELL, f32::from(lines) * Host::CELL)
            }
        }
    }

    fn event(&self, change: Change) -> Option<Event> {
        let (width, height) = self.view();
        let at = Point::new(width / 2.0, height / 2.0);
        match change {
            Change::Point => Some(Event::pointer_move(at)),
            Change::Wheel(lines) => Some(Event::wheel(at, lines)),
            _ => None,
        }
    }

    fn present(&mut self, event: Option<Event>) {
        let cells = self.cells();
        let screen = self.screen.as_mut().expect("the screen side owns a screen");
        screen.set_dt(FRAME_SECONDS);
        screen.frame(&mut self.host, event, cells).expect("layout");
        self.out.clear();
        screen.flush(&mut self.out).expect("flush");
    }
}

impl Rig for SigilRig {
    fn frame(&mut self, change: Change) {
        if let Some(narrow) = change.narrow() {
            self.narrow = narrow;
            if let Some(screen) = &mut self.screen {
                screen.invalidate();
            }
        }
        if matches!(change, Change::Grow) {
            self.stream.update(extend);
        }
        let event = self.event(change);
        match self.surface {
            Surface::Scene => {
                if let Some(event) = event {
                    self.host.route(event);
                }
                black_box(self.host.scene(self.view()).expect("layout"));
            }
            Surface::Grid => {
                let (cols, lines) = self.cells();
                black_box(self.host.grid(event, cols, lines).expect("layout"));
            }
            Surface::Screen => {
                self.present(event);
                black_box(self.out.len());
            }
        }
    }

    fn show(&mut self) -> String {
        match self.surface {
            Surface::Scene => {
                let scene = self.host.scene(self.view()).expect("layout");
                let runs: Vec<&str> = scene.glyph_runs().map(|run| run.text()).collect();
                format!("glyph runs {}: {:?}", runs.len(), runs.first())
            }
            Surface::Grid => {
                let (cols, lines) = self.cells();
                self.host.grid(None, cols, lines).expect("layout").text()
            }
            Surface::Screen => {
                self.present(None);
                let text = self.screen.as_ref().map(Screen::text).unwrap_or_default();
                format!("{text}\nflushed bytes {}", self.out.len())
            }
        }
    }

    fn scoped(run: impl FnOnce()) {
        Owner::scope("rivals", run);
    }
}
