use sigil::Host;
use sigil::events::EventRouter;
use sigil::kernel::{container, text};
use sigil::layout::{LayoutStyle, Position};
use sigil::prelude::LayoutBuilderExt;
use sigil::reactive::{Runtime, signal};
use sigil::terminal::Screen;

use super::oracle::{COLS, Model, ROWS, Spot, ansi, tree, truth};

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
        let _profiled = crate::profiled();
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

#[test]
fn two_frames_before_one_flush_keep_both_changes() {
    let shown = Runtime::scope("oracle-unflushed", || {
        let mut model = Model {
            lines: (0..ROWS).map(|row| format!("row {row}")).collect(),
            ..Model::default()
        };
        let state = signal(model.clone());
        let mut host = Host::cells();
        let mut router = EventRouter::new();
        host.mount(move || tree(state));
        let mut screen = Screen::new(COLS, ROWS);
        for _ in 0..2 {
            screen
                .frame(&mut host, &mut router, [], (COLS, ROWS))
                .expect("frame");
            screen.flush(&mut std::io::sink()).expect("flush");
        }
        model.lines[1] = String::from("first");
        state.set(model.clone());
        screen
            .frame(&mut host, &mut router, [], (COLS, ROWS))
            .expect("frame");
        model.lines[9] = String::from("second");
        state.set(model.clone());
        let grid = screen
            .frame(&mut host, &mut router, [], (COLS, ROWS))
            .expect("frame");
        (String::from_utf8_lossy(&ansi(grid)).into_owned(), model)
    });
    let (incremental, model) = shown;
    let fresh = String::from_utf8_lossy(&truth(&model)).into_owned();
    assert_eq!(incremental, fresh, "an unflushed frame lost its change");
}

#[test]
fn an_overflowing_word_clears_when_it_changes() {
    let cell = Host::CELL;
    let (incremental, fresh) = Runtime::scope("oracle-overflow", || {
        let word = signal(String::from("abcdefghij"));
        let tree = move || {
            container()
                .layout(LayoutStyle::block().fill_width().fill_height())
                .child(
                    container()
                        .layout(
                            LayoutStyle::flex_row()
                                .position(Position::Absolute)
                                .inset_left(4.0 * cell)
                                .inset_top(2.0 * cell)
                                .width(cell)
                                .height(cell),
                        )
                        .child(text(word).nowrap()),
                )
        };
        let mut host = Host::cells();
        let mut router = EventRouter::new();
        host.mount(tree);
        let mut screen = Screen::new(COLS, ROWS);
        for _ in 0..2 {
            screen
                .frame(&mut host, &mut router, [], (COLS, ROWS))
                .expect("frame");
            screen.flush(&mut std::io::sink()).expect("flush");
        }
        assert!(
            screen.text().contains("abcdefghij"),
            "the long word did not overflow its cell: {}",
            screen.text()
        );
        word.set(String::from("z"));
        let incremental = screen
            .frame(&mut host, &mut router, [], (COLS, ROWS))
            .expect("frame")
            .text();
        let mut fresh_host = Host::cells();
        fresh_host.mount(tree);
        let fresh = Screen::new(COLS, ROWS)
            .frame(&mut fresh_host, &mut router, [], (COLS, ROWS))
            .expect("frame")
            .text();
        (incremental, fresh)
    });
    assert_eq!(
        incremental, fresh,
        "an overflowing word left old cells behind"
    );
}
