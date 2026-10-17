use proptest::prelude::*;
use sigil::Host;
use sigil::events::EventRouter;
use sigil::kernel::{IntoNode, container, text};
use sigil::layout::{LayoutStyle, Position};
use sigil::prelude::{LayoutBuilderExt, Paint, Signal};
use sigil::reactive::{Runtime, derived, signal};
use sigil::style::{NodeStyleExt, Rgba8, Token};
use sigil::terminal::{ColorDepth, Grid, Screen, render};

const COLS: u16 = 40;
const ROWS: u16 = 12;
const INKS: [Rgba8; 4] = [
    Rgba8 {
        r: 250,
        g: 250,
        b: 250,
        a: 255,
    },
    Rgba8 {
        r: 120,
        g: 160,
        b: 250,
        a: 255,
    },
    Rgba8 {
        r: 250,
        g: 120,
        b: 90,
        a: 255,
    },
    Rgba8 {
        r: 90,
        g: 220,
        b: 140,
        a: 255,
    },
];

#[derive(Clone, Debug, PartialEq)]
struct Spot {
    x: u16,
    y: u16,
    word: String,
    ink: usize,
}

#[derive(Clone, Debug, Default, PartialEq)]
struct Model {
    spots: Vec<Spot>,
    lines: Vec<String>,
    accent: bool,
}

#[derive(Clone, Debug)]
enum Edit {
    Add(Spot),
    Move(usize, u16, u16),
    Recolor(usize, usize),
    Reword(usize, String),
    Remove(usize),
    Line(usize, String),
    Push(String),
    Pop,
    Accent,
}

fn word() -> impl Strategy<Value = String> {
    "[a-z]{1,6}"
}

fn spot() -> impl Strategy<Value = Spot> {
    (0..COLS - 8, 0..ROWS, word(), 0..INKS.len()).prop_map(|(x, y, word, ink)| Spot {
        x,
        y,
        word,
        ink,
    })
}

fn edit() -> impl Strategy<Value = Edit> {
    prop_oneof![
        spot().prop_map(Edit::Add),
        (0..8_usize, 0..COLS - 8, 0..ROWS).prop_map(|(at, x, y)| Edit::Move(at, x, y)),
        (0..8_usize, 0..INKS.len()).prop_map(|(at, ink)| Edit::Recolor(at, ink)),
        (0..8_usize, word()).prop_map(|(at, word)| Edit::Reword(at, word)),
        (0..8_usize).prop_map(Edit::Remove),
        (0..4_usize, word()).prop_map(|(at, word)| Edit::Line(at, word)),
        word().prop_map(Edit::Push),
        Just(Edit::Pop),
        Just(Edit::Accent),
    ]
}

fn apply(model: &mut Model, edit: Edit) {
    let spots = model.spots.len();
    let lines = model.lines.len();
    match edit {
        Edit::Add(spot) => model.spots.push(spot),
        Edit::Move(at, x, y) if spots > 0 => {
            let spot = &mut model.spots[at % spots];
            (spot.x, spot.y) = (x, y);
        }
        Edit::Recolor(at, ink) if spots > 0 => model.spots[at % spots].ink = ink,
        Edit::Reword(at, word) if spots > 0 => model.spots[at % spots].word = word,
        Edit::Remove(at) if spots > 0 => {
            model.spots.remove(at % spots);
        }
        Edit::Line(at, word) if lines > 0 => model.lines[at % lines] = word,
        Edit::Push(word) => model.lines.push(word),
        Edit::Pop => {
            model.lines.pop();
        }
        Edit::Accent => model.accent = !model.accent,
        _ => {}
    }
}

fn placed(spot: &Spot) -> impl IntoNode + use<> {
    let cell = Host::CELL;
    container()
        .layout(
            LayoutStyle::flex_row()
                .position(Position::Absolute)
                .inset_left(f32::from(spot.x) * cell)
                .inset_top(f32::from(spot.y) * cell)
                .width(spot.word.chars().count() as f32 * cell)
                .height(cell),
        )
        .child(text(spot.word.clone()).nowrap().text_color(INKS[spot.ink]))
}

fn tree(model: Signal<Model>) -> impl IntoNode {
    let accent = derived(move || model.with(|model| model.accent));
    container()
        .layout(LayoutStyle::block().fill_width().fill_height())
        .paint_when(accent, |style| style.bg(Token::Accent))
        .child(
            container()
                .layout(LayoutStyle::flex_column().fill_width())
                .children_reactive(move || {
                    model.with(|model| {
                        model
                            .lines
                            .iter()
                            .map(|line| text(line.clone()).nowrap())
                            .collect::<Vec<_>>()
                    })
                }),
        )
        .child(
            container()
                .layout(
                    LayoutStyle::block()
                        .position(Position::Absolute)
                        .inset(0.0)
                        .fill_width()
                        .fill_height(),
                )
                .children_reactive(move || {
                    model.with(|model| model.spots.iter().map(placed).collect::<Vec<_>>())
                }),
        )
}

fn ansi(grid: &Grid) -> Vec<u8> {
    let mut out = Vec::new();
    render(&mut out, &Grid::new(0, 0), grid, ColorDepth::Truecolor).expect("render");
    out
}

fn truth(model: &Model) -> Vec<u8> {
    Runtime::scope("oracle-truth", || {
        let state = signal(model.clone());
        let mut host = Host::cells();
        let mut router = EventRouter::new();
        host.mount(move || tree(state));
        let mut screen = Screen::new(COLS, ROWS);
        let grid = screen
            .frame(&mut host, &mut router, [], (COLS, ROWS))
            .expect("frame");
        ansi(grid)
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(96))]

    #[test]
    fn an_incremental_frame_equals_a_frame_built_from_scratch(
        edits in proptest::collection::vec(edit(), 1..24),
    ) {
        let mismatch = diverged(edits);
        prop_assert!(mismatch.is_none(), "{}", mismatch.unwrap_or_default());
    }
}

fn diverged(edits: Vec<Edit>) -> Option<String> {
    Runtime::scope("oracle-incremental", || {
        let mut model = Model {
            lines: vec!["top".to_string()],
            ..Model::default()
        };
        let state = signal(model.clone());
        let mut host = Host::cells();
        let mut router = EventRouter::new();
        host.mount(move || tree(state));
        let mut screen = Screen::new(COLS, ROWS);
        for (step, edit) in edits.into_iter().enumerate() {
            apply(&mut model, edit);
            state.set(model.clone());
            let grid = screen
                .frame(&mut host, &mut router, [], (COLS, ROWS))
                .expect("frame");
            let incremental = String::from_utf8_lossy(&ansi(grid)).into_owned();
            screen.flush(&mut std::io::sink()).expect("flush");
            let fresh = String::from_utf8_lossy(&truth(&model)).into_owned();
            if incremental != fresh {
                return Some(format!(
                    "frame {step} diverged for {model:?}\nincremental: {incremental:?}\nfresh:       {fresh:?}"
                ));
            }
        }
        None
    })
}

fn frames(steps: &[&[&str]]) -> Vec<String> {
    Runtime::scope("oracle-lines", || {
        let lines = signal(steps[0].iter().map(ToString::to_string).collect::<Vec<_>>());
        let mut host = Host::cells();
        let mut router = EventRouter::new();
        host.mount(move || {
            container()
                .layout(LayoutStyle::flex_column().fill_width())
                .children_reactive(move || {
                    lines.with(|lines| {
                        lines
                            .iter()
                            .map(|line| text(line.clone()).nowrap())
                            .collect::<Vec<_>>()
                    })
                })
        });
        let mut screen = Screen::new(COLS, ROWS);
        steps
            .iter()
            .map(|step| {
                lines.set(step.iter().map(ToString::to_string).collect());
                screen
                    .frame(&mut host, &mut router, [], (COLS, ROWS))
                    .expect("frame")
                    .text()
            })
            .collect()
    })
}

#[test]
fn a_patched_text_row_keeps_its_text() {
    let shown = frames(&[&["top"], &["a"], &["a", "a"], &["b", "a"]]);
    let first = |frame: &String| {
        frame
            .lines()
            .take(2)
            .map(|line| line.trim_end().to_string())
            .collect::<Vec<_>>()
    };
    assert_eq!(first(&shown[0]), ["top", ""]);
    assert_eq!(first(&shown[1]), ["a", ""]);
    assert_eq!(first(&shown[2]), ["a", "a"]);
    assert_eq!(first(&shown[3]), ["b", "a"]);
}

#[test]
fn a_patched_text_node_emits_its_new_text() {
    Runtime::scope("oracle-emit", || {
        let lines = signal(vec!["top".to_string()]);
        let mut host = Host::cells();
        host.mount(move || {
            container()
                .layout(LayoutStyle::flex_column().fill_width())
                .children_reactive(move || {
                    lines.with(|lines| {
                        lines
                            .iter()
                            .map(|line| text(line.clone()))
                            .collect::<Vec<_>>()
                    })
                })
        });
        host.layout((320.0, 96.0)).expect("layout");
        let first = host.emit((320.0, 96.0));
        assert!(first.find_text("top").is_some(), "{first:?}");
        lines.set(vec!["a".to_string()]);
        host.layout((320.0, 96.0)).expect("layout");
        let second = host.emit((320.0, 96.0));
        assert!(second.find_text("a").is_some(), "{second:?}");
    });
}

#[test]
fn a_moved_spot_rasters_only_its_window() {
    use sigil::profile::{self, BackendStatus, Counter};
    let rastered = Runtime::scope("oracle-counters", || {
        let mut model = Model {
            lines: (0..ROWS)
                .map(|row| format!("row {row} steady text"))
                .collect(),
            spots: vec![Spot {
                x: 2,
                y: 2,
                word: "star".to_string(),
                ink: 1,
            }],
            accent: false,
        };
        let state = signal(model.clone());
        let mut host = Host::cells();
        let mut router = EventRouter::new();
        host.mount(move || tree(state));
        let mut screen = Screen::new(COLS, ROWS);
        let mut sink = std::io::sink();
        for _ in 0..2 {
            screen
                .frame(&mut host, &mut router, [], (COLS, ROWS))
                .expect("frame");
            screen.flush(&mut sink).expect("flush");
        }
        profile::enable();
        profile::reset();
        profile::begin_frame(BackendStatus::TERMINAL);
        model.spots[0].x = 20;
        state.set(model.clone());
        screen
            .frame(&mut host, &mut router, [], (COLS, ROWS))
            .expect("frame");
        screen.flush(&mut sink).expect("flush");
        profile::tick_frame();
        let counted = profile::frames()
            .last()
            .map_or(u64::MAX, |frame| frame.counter(Counter::TuiCellsRastered));
        profile::disable();
        counted
    });
    let screen = u64::from(COLS) * u64::from(ROWS);
    assert!(rastered > 0, "the move was not rastered at all");
    assert!(
        rastered <= 56,
        "a one-word move rastered {rastered} of {screen} cells; its two 7x4 windows hold 56"
    );
}
