use proptest::prelude::*;
use sigil::Host;
use sigil::events::{EventRouter, ShellEvent};
use sigil::kernel::{IntoNode, container, text};
use sigil::layout::LayoutStyle;
use sigil::prelude::{LayoutBuilderExt, Paint, Signal};
use sigil::reactive::{Runtime, signal};
use sigil::style::{StateVariants, Token};
use std::fmt::Write as _;
use std::time::Duration;

use sigil::terminal::{Access, ColorDepth, Grid, Screen, render};
use sigil::widgets::{button, modal, text_edit, virtual_list};
use sigil_kittest::input::{pointer_move, scroll_lines, tab, type_char};

const CELL: f32 = Host::CELL;

#[derive(Clone, Debug)]
enum Step {
    Row(usize, String),
    Push(String),
    Pop,
    Scroll(f32),
    Tab,
    Type(char),
    Hover(u16, u16),
    Modal,
    Status(String),
    Resize(u16, u16),
}

#[derive(Clone, Copy)]
struct State {
    rows: Signal<Vec<String>>,
    status: Signal<String>,
    shown: Signal<bool>,
}

fn word() -> impl Strategy<Value = String> {
    "[a-z]{1,8}"
}

fn step() -> impl Strategy<Value = Step> {
    prop_oneof![
        (0..40_usize, word()).prop_map(|(at, word)| Step::Row(at, word)),
        word().prop_map(Step::Push),
        Just(Step::Pop),
        prop_oneof![Just(-3.0_f32), Just(3.0_f32)].prop_map(Step::Scroll),
        Just(Step::Tab),
        proptest::char::range('a', 'z').prop_map(Step::Type),
        (0..48_u16, 0..16_u16).prop_map(|(x, y)| Step::Hover(x, y)),
        Just(Step::Modal),
        word().prop_map(Step::Status),
        (24..60_u16, 8..20_u16).prop_map(|(cols, rows)| Step::Resize(cols, rows)),
    ]
}

fn tree(state: State) -> impl IntoNode {
    container()
        .layout(LayoutStyle::flex_column().fill_width().fill_height())
        .child(text(state.status))
        .child(
            container()
                .layout(LayoutStyle::flex_row().gap(CELL))
                .child(button("first"))
                .child(button("second"))
                .child(
                    sigil::widgets::interactive(container().child(text("hover me")))
                        .hover(|style| style.bg(Token::Accent)),
                ),
        )
        .child(text_edit(signal(String::from("edit"))))
        .child(text_edit(signal(String::from("note"))))
        .child(
            virtual_list(state.rows, |_, row: &String| {
                container().child(text(row.clone()))
            })
            .row_height(CELL)
            .viewport_height(CELL * 6.0)
            .viewport_width(CELL * 40.0),
        )
        .child(modal(state.shown).title("Saved"))
}

fn events(step: &Step) -> Vec<ShellEvent> {
    match step {
        Step::Scroll(lines) => vec![scroll_lines(CELL * 4.0, CELL * 6.0, *lines)],
        Step::Tab => vec![tab(false)],
        Step::Type(ch) => vec![type_char(
            &ch.to_string(),
            sigil::events::PhysicalKey::Unidentified,
        )],
        Step::Hover(x, y) => vec![pointer_move(f32::from(*x) * CELL, f32::from(*y) * CELL)],
        _ => Vec::new(),
    }
}

fn apply(state: State, step: &Step, size: &mut (u16, u16)) {
    match step {
        Step::Row(at, word) => state.rows.update(|rows| {
            let len = rows.len();
            if len > 0 {
                rows[at % len].clone_from(word);
            }
        }),
        Step::Push(word) => state.rows.update(|rows| rows.push(word.clone())),
        Step::Pop => state.rows.update(|rows| {
            rows.pop();
        }),
        Step::Modal => state.shown.update(|shown| *shown = !*shown),
        Step::Status(word) => state.status.set(word.clone()),
        Step::Resize(cols, rows) => *size = (*cols, *rows),
        _ => {}
    }
}

fn ansi(grid: &Grid) -> String {
    let mut out = Vec::new();
    render(&mut out, &Grid::new(0, 0), grid, ColorDepth::Truecolor).expect("render");
    String::from_utf8_lossy(&out).into_owned()
}

pub(super) struct Side {
    host: Host,
    router: EventRouter,
    screen: Screen,
    term: vt100::Parser,
}

impl Side {
    fn new(state: State, size: (u16, u16), status: Duration) -> Self {
        let mut host = Host::cells();
        host.mount(move || tree(state));
        Self::mounted(host, size, status)
    }

    pub(super) fn mounted(host: Host, size: (u16, u16), status: Duration) -> Self {
        let mut screen = Screen::new(size.0, size.1);
        screen.set_access(Access {
            status,
            ..Access::default()
        });
        Self {
            host,
            router: EventRouter::new(),
            screen,
            term: vt100::Parser::new(size.1, size.0, 0),
        }
    }

    pub(super) fn frame(
        &mut self,
        events: Vec<ShellEvent>,
        size: (u16, u16),
        full: bool,
    ) -> String {
        if full {
            self.host.reset_emit();
            self.screen.invalidate();
        }
        let grid = self
            .screen
            .frame(&mut self.host, &mut self.router, events, size)
            .expect("frame");
        let (cells, cursor) = (ansi(grid), grid.cursor());
        let mut bytes = Vec::new();
        self.screen.flush(&mut bytes).expect("flush");
        self.term.screen_mut().set_size(size.1, size.0);
        self.term.process(&bytes);
        let term = self.term.screen();
        format!(
            "{cells}\ncursor {cursor:?}\nterminal {:?}:\n{}",
            (!term.hide_cursor()).then(|| term.cursor_position()),
            seen(term, size)
        )
    }
}

pub(super) fn seen(term: &vt100::Screen, (cols, rows): (u16, u16)) -> String {
    let mut shown = String::new();
    for row in 0..rows {
        for col in 0..cols {
            let Some(cell) = term.cell(row, col) else {
                continue;
            };
            let text = if cell.has_contents() {
                cell.contents()
            } else {
                " "
            };
            let _ = write!(
                shown,
                "{text}{:?}{:?}{}{}{}{}|",
                cell.fgcolor(),
                cell.bgcolor(),
                u8::from(cell.bold()),
                u8::from(cell.italic()),
                u8::from(cell.underline()),
                u8::from(cell.inverse()),
            );
        }
        shown.push('\n');
    }
    shown
}

fn state() -> State {
    State {
        rows: signal((0..30).map(|row| format!("row {row}")).collect()),
        status: signal(String::from("ready")),
        shown: signal(false),
    }
}

fn frames(steps: &[Step], full: bool) -> Vec<String> {
    Runtime::scope("live-oracle", || {
        let shared = state();
        let mut size = (48_u16, 16_u16);
        let mut host = Side::new(shared, size, Access::default().status);
        for _ in 0..2 {
            host.frame(Vec::new(), size, full);
        }
        steps
            .iter()
            .map(|step| {
                apply(shared, step, &mut size);
                host.frame(events(step), size, full)
            })
            .collect()
    })
}

fn diverged(steps: &[Step]) -> Option<String> {
    let quick = frames(steps, false);
    let whole = frames(steps, true);
    quick
        .iter()
        .zip(&whole)
        .enumerate()
        .find(|(_, (quick, whole))| quick != whole)
        .map(|(index, (quick, whole))| {
            format!(
                "step {index} {:?} diverged\nincremental: {quick:?}\nfull:        {whole:?}",
                steps[index]
            )
        })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn an_interactive_incremental_frame_equals_a_full_repaint(
        steps in proptest::collection::vec(step(), 1..32),
    ) {
        let mismatch = diverged(&steps);
        prop_assert!(mismatch.is_none(), "{}", mismatch.unwrap_or_default());
    }
}

fn expiring(full: bool) -> Vec<String> {
    Runtime::scope("live-status", || {
        let shared = state();
        let mut size = (48_u16, 16_u16);
        let status = Duration::from_millis(20);
        let mut host = Side::new(shared, size, status);
        let steps = [
            Step::Status(String::from("ready")),
            Step::Modal,
            Step::Status(String::from("next")),
            Step::Status(String::from("more")),
            Step::Modal,
        ];
        steps
            .iter()
            .map(|step| {
                std::thread::sleep(status * 3);
                apply(shared, step, &mut size);
                host.frame(Vec::new(), size, full)
            })
            .collect()
    })
}

#[test]
fn an_expired_status_clears_on_a_damaged_frame() {
    let quick = expiring(false);
    assert!(
        quick.last().is_some_and(|last| !last.contains("Saved")),
        "the expired status stayed on screen"
    );
    assert_eq!(quick, expiring(true));
}

const SETTLE: usize = 30;

fn edit() -> impl Strategy<Value = Step> {
    prop_oneof![
        (0..40_usize, word()).prop_map(|(at, word)| Step::Row(at, word)),
        word().prop_map(Step::Push),
        Just(Step::Pop),
        word().prop_map(Step::Status),
        (24..60_u16, 8..20_u16).prop_map(|(cols, rows)| Step::Resize(cols, rows)),
    ]
}

fn settled(state: State, size: (u16, u16)) -> String {
    let mut host = Side::new(state, size, Duration::ZERO);
    for _ in 0..SETTLE {
        host.frame(Vec::new(), size, false);
    }
    ansi(
        host.screen
            .frame(&mut host.host, &mut host.router, [], size)
            .expect("frame"),
    )
}

fn edited(steps: &[Step]) -> (String, String) {
    let incremental = Runtime::scope("live-edited", || {
        let shared = state();
        let mut size = (48_u16, 16_u16);
        let mut host = Side::new(shared, size, Duration::ZERO);
        for _ in 0..2 {
            host.frame(Vec::new(), size, false);
        }
        for step in steps {
            apply(shared, step, &mut size);
            host.frame(Vec::new(), size, false);
        }
        for _ in 0..SETTLE {
            host.frame(Vec::new(), size, false);
        }
        let grid = host
            .screen
            .frame(&mut host.host, &mut host.router, [], size)
            .expect("frame");
        (ansi(grid), size)
    });
    let (shown, size) = incremental;
    let fresh = Runtime::scope("live-mounted", || {
        let shared = state();
        let mut ignored = (48_u16, 16_u16);
        for step in steps {
            apply(shared, step, &mut ignored);
        }
        settled(shared, size)
    });
    (shown, fresh)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn edits_to_a_live_tree_match_a_tree_mounted_at_the_end(
        steps in proptest::collection::vec(edit(), 1..24),
    ) {
        let (incremental, fresh) = edited(&steps);
        prop_assert_eq!(incremental, fresh);
    }
}

#[test]
fn an_editor_squeezed_by_a_resize_matches_one_mounted_at_that_size() {
    let (incremental, fresh) = edited(&[Step::Resize(24, 8)]);
    assert_eq!(incremental, fresh);
}
