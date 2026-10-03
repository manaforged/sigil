use super::*;
use sigil_kittest::Harness;

#[test]
fn same_tree_paints_label() {
    let grid = Harness::new(counter).size(40, 12).text();
    assert!(grid.contains("Count: 0"), "expected Count: 0 in:\n{grid}");
    assert!(grid.contains("Increment"), "expected Increment in:\n{grid}");
}

#[test]
fn enter_increments_the_same_button() {
    let grid = Harness::new(counter).size(40, 12).press(Key::Enter).text();
    assert!(
        grid.contains("Count: 1"),
        "expected Count: 1 after Enter in:\n{grid}"
    );
}

#[test]
fn progress_paints_a_block_bar() {
    use sigil::reactive::signal;
    use sigil::widgets::progress;
    let grid = present(
        || {
            let value = signal(0.6_f32);
            progress(value, 1.0)
        },
        40,
        8,
        &[],
    );
    let filled = grid.matches('█').count();
    let empty = present(
        || {
            let value = signal(0.0_f32);
            progress(value, 1.0)
        },
        40,
        8,
        &[],
    )
    .matches('█')
    .count();
    let full = present(
        || {
            let value = signal(1.0_f32);
            progress(value, 1.0)
        },
        40,
        8,
        &[],
    )
    .matches('█')
    .count();
    assert!(
        empty < filled && filled < full,
        "0.6 must sit between 0.0 and 1.0, got {empty} < {filled} < {full}"
    );
}

#[test]
fn chart_bars_use_the_same_block_fill() {
    use sigil::widgets::chart;
    let grid = present(|| chart([1.0, 3.0, 2.0]).bar(), 40, 16, &[]);
    assert!(grid.contains('█'), "expected chart bars in:\n{grid}");
}

#[test]
fn mermaid_fence_paints_boxes() {
    use sigil::reactive::signal;
    use sigil::widgets::markdown;
    let grid = present(
        || {
            let src = signal("```mermaid\ngraph TD\nA[Go] --> B[Stop]\n```".to_string());
            markdown(src)
        },
        40,
        16,
        &[],
    );
    assert!(grid.contains("Go"), "expected Go in:\n{grid}");
    assert!(grid.contains("Stop"), "expected Stop in:\n{grid}");
    assert!(grid.contains('┌'), "expected a box in:\n{grid}");
}

#[test]
fn a_chip_cell_paints_on_its_own_row() {
    use sigil::style::Tone;
    use sigil::widgets::{ColWidth, TableColumn, TableState, chip, table};

    #[derive(Clone)]
    struct Row {
        name: &'static str,
    }

    let grid = present(
        || {
            let rows = vec![Row { name: "aaa" }, Row { name: "bbb" }];
            let state = TableState::new(
                rows,
                vec![
                    TableColumn::new("name", "NAME")
                        .width(ColWidth::Fixed(64.0))
                        .cell(|row: &Row| text(row.name).nowrap()),
                    TableColumn::new("tag", "TAG")
                        .width(ColWidth::Fixed(64.0))
                        .cell(|row: &Row| chip(row.name, Tone::Neutral)),
                ],
            );
            container().child(text("heading")).child(
                table(state)
                    .row_height(8.0)
                    .header_height(8.0)
                    .viewport_height(64.0),
            )
        },
        60,
        14,
        &[],
    );
    for name in ["aaa", "bbb"] {
        let line = grid
            .lines()
            .find(|line| line.contains(name))
            .unwrap_or_else(|| panic!("no line for {name} in:\n{grid}"));
        assert_eq!(
            line.matches(name).count(),
            2,
            "label and chip must share one row in:\n{grid}"
        );
    }
}

#[test]
fn a_wide_cell_clips_at_the_column_edge() {
    use sigil::widgets::{ColWidth, TableColumn, TableState, table};

    #[derive(Clone)]
    struct Row;

    let grid = present(
        || {
            let state = TableState::new(
                vec![Row],
                vec![
                    TableColumn::new("wide", "WIDE")
                        .width(ColWidth::Fixed(32.0))
                        .cell(|_: &Row| text("aaaaaaaaaaaaaaaa").nowrap()),
                    TableColumn::new("tail", "TAIL")
                        .width(ColWidth::Fixed(64.0))
                        .cell(|_: &Row| text("tail").nowrap()),
                ],
            );
            table(state)
                .row_height(8.0)
                .header_height(8.0)
                .viewport_height(32.0)
        },
        60,
        10,
        &[],
    );
    let row = grid
        .lines()
        .find(|line| line.contains('a'))
        .unwrap_or_else(|| panic!("no data row in:\n{grid}"));
    assert!(
        row.matches('a').count() <= 4,
        "the value must be cut at the column edge, got {row:?} in:\n{grid}"
    );
    assert!(
        row.contains("tail"),
        "the neighbour still paints in:\n{grid}"
    );
}

#[test]
fn a_button_surface_is_not_a_block_slab() {
    let grid = present(|| button("Go"), 30, 8, &[]);
    assert!(grid.contains("Go"), "expected the label in:\n{grid}");
    assert!(
        !grid.contains('\u{2588}'),
        "a button holds its label, so it tints rather than filling:\n{grid}"
    );
}

#[test]
fn a_control_stands_one_line_tall() {
    let grid = present(
        || {
            container()
                .layout(sigil::layout::LayoutStyle::flex_column())
                .child(button("One"))
                .child(button("Two"))
        },
        30,
        10,
        &[],
    );
    let lines: Vec<&str> = grid.lines().collect();
    let first = lines
        .iter()
        .position(|line| line.contains("One"))
        .unwrap_or_else(|| panic!("no One in:\n{grid}"));
    let second = lines
        .iter()
        .position(|line| line.contains("Two"))
        .unwrap_or_else(|| panic!("no Two in:\n{grid}"));
    assert_eq!(
        second - first,
        1,
        "stacked controls must sit on consecutive lines in:\n{grid}"
    );
}

#[test]
fn a_filled_table_keeps_its_header_off_row_one() {
    use sigil::widgets::{ColWidth, TableColumn, TableState, table};

    #[derive(Clone)]
    struct Row {
        name: &'static str,
    }

    let grid = present(
        || {
            let state = TableState::new(
                vec![Row { name: "aaa" }, Row { name: "bbb" }],
                vec![
                    TableColumn::new("n", "NAME")
                        .width(ColWidth::Fixed(64.0))
                        .cell(|row: &Row| text(row.name).nowrap()),
                ],
            );
            table(state)
                .row_height(8.0)
                .header_height(8.0)
                .viewport_fill()
        },
        40,
        12,
        &[],
    );
    let header = grid
        .lines()
        .position(|line| line.contains("NAME"))
        .unwrap_or_else(|| panic!("no header in:\n{grid}"));
    let first = grid
        .lines()
        .position(|line| line.contains("aaa"))
        .unwrap_or_else(|| panic!("no first row in:\n{grid}"));
    assert!(
        first > header,
        "row one must sit below the header in:\n{grid}"
    );
}

#[test]
fn a_timer_comes_due_on_the_frame_clock() {
    use std::cell::Cell as StdCell;
    use std::rc::Rc;
    use std::time::{Duration, Instant};

    use sigil::Host;
    use sigil::events::EventRouter;

    let ticks = Rc::new(StdCell::new(0_u32));
    let counter = Rc::clone(&ticks);
    sigil::reactive::Runtime::scope("timer", || {
        let mut host = Host::cells();
        let mut router = EventRouter::new();
        host.mount(move || text("tick"));
        sigil::reactive::__test_set_clock(Instant::now());
        sigil::reactive::interval(Duration::from_millis(99), move || {
            counter.set(counter.get() + 1);
        });
        for _ in 0..9 {
            sigil::reactive::__test_advance_clock(Duration::from_millis(33));
            drop(step(&mut host, &mut router, [], 20, 4));
        }
    });
    sigil::reactive::__test_clear_clock();
    assert_eq!(ticks.get(), 3, "a 99 ms interval fires 3 times in 9 frames");
}

#[test]
fn a_wide_glyph_keeps_the_next_column() {
    use unicode_width::UnicodeWidthChar;

    use sigil::widgets::{ColWidth, TableColumn, TableState, table};

    #[derive(Clone)]
    struct Row {
        name: &'static str,
        tail: &'static str,
    }

    let grid = present(
        || {
            let state = TableState::new(
                vec![
                    Row {
                        name: "ab",
                        tail: "L",
                    },
                    Row {
                        name: "\u{6f22}\u{5b57}",
                        tail: "R",
                    },
                ],
                vec![
                    TableColumn::new("n", "NAME")
                        .width(ColWidth::Fixed(64.0))
                        .cell(|row: &Row| text(row.name).nowrap()),
                    TableColumn::new("t", "TAIL")
                        .width(ColWidth::Fixed(64.0))
                        .cell(|row: &Row| text(row.tail).nowrap()),
                ],
            );
            table(state)
                .row_height(8.0)
                .header_height(8.0)
                .viewport_height(32.0)
        },
        40,
        10,
        &[],
    );
    let column = |mark: char| -> Option<usize> {
        grid.lines()
            .filter(|line| !line.contains("NAME"))
            .find_map(|line| {
                let mut col = 0;
                for ch in line.chars() {
                    if ch == mark {
                        return Some(col);
                    }
                    col += UnicodeWidthChar::width(ch).unwrap_or(0);
                }
                None
            })
    };
    assert_eq!(
        column('L'),
        column('R'),
        "the CJK row must not shift its neighbour:\n{grid}"
    );
    assert!(column('L').is_some(), "expected both markers in:\n{grid}");
}

#[test]
fn a_combining_mark_keeps_its_base() {
    let marked = present(|| text("cafe\u{301}|"), 20, 3, &[]);
    let plain = present(|| text("cafe|"), 20, 3, &[]);
    let bar = |grid: &str| -> Option<usize> {
        grid.lines().next().and_then(|line| {
            let mut col = 0;
            for ch in line.chars() {
                if ch == '|' {
                    return Some(col);
                }
                col += unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
            }
            None
        })
    };
    assert_eq!(
        bar(&marked),
        bar(&plain),
        "a combining mark must not take a column of its own:\n{marked}"
    );
}
