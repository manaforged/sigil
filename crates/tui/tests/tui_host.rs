#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::too_many_lines,
    missing_docs
)]

use sigil::hal::{Key, ShellEvent};
use sigil::kernel::{Cx, NodeId, container, text};
use sigil::reactive::{derived, signal};
use sigil::widgets::button;

use sigil_tui::{key_down, present as try_present};

fn present(tree: impl sigil_tui::TreeRoot, cols: u16, rows: u16, events: &[ShellEvent]) -> String {
    try_present(tree, cols, rows, events).expect("layout")
}

fn step(
    host: &mut sigil_tui::Host,
    router: &mut sigil_tui::EventRouter,
    events: impl IntoIterator<Item = ShellEvent>,
    cols: u16,
    rows: u16,
) -> sigil_tui::Grid {
    sigil_tui::step(host, router, events, cols, rows).expect("layout")
}

fn counter(cx: &mut Cx) -> NodeId {
    let count = signal(0_i32);
    let label = derived(move || format!("Count: {}", count.get()));
    cx.add(
        container()
            .child(text(label))
            .child(button("Increment").activate(move || {
                count.set(count.peek() + 1);
            })),
    )
}

#[test]
fn same_tree_paints_label() {
    let grid = present(counter, 40, 12, &[]);
    assert!(grid.contains("Count: 0"), "expected Count: 0 in:\n{grid}");
    assert!(grid.contains("Increment"), "expected Increment in:\n{grid}");
}

#[test]
fn enter_increments_the_same_button() {
    let grid = present(counter, 40, 12, &[key_down(Key::Enter)]);
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
        |cx: &mut Cx| {
            let value = signal(0.6_f32);
            cx.add(progress(value, 1.0))
        },
        40,
        8,
        &[],
    );
    let filled = grid.matches('█').count();
    let empty = present(
        |cx: &mut Cx| {
            let value = signal(0.0_f32);
            cx.add(progress(value, 1.0))
        },
        40,
        8,
        &[],
    )
    .matches('█')
    .count();
    let full = present(
        |cx: &mut Cx| {
            let value = signal(1.0_f32);
            cx.add(progress(value, 1.0))
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
    let grid = present(
        |cx: &mut Cx| cx.add(chart([1.0, 3.0, 2.0]).bar()),
        40,
        16,
        &[],
    );
    assert!(grid.contains('█'), "expected chart bars in:\n{grid}");
}

#[test]
fn mermaid_fence_paints_boxes() {
    use sigil::reactive::signal;
    use sigil::widgets::markdown;
    let grid = present(
        |cx: &mut Cx| {
            let src = signal("```mermaid\ngraph TD\nA[Go] --> B[Stop]\n```".to_string());
            cx.add(markdown(src))
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
    use sigil::widgets::{ColWidth, TableColumn, TableState, badge, label, table};

    #[derive(Clone)]
    struct Row {
        name: &'static str,
    }

    let grid = present(
        |cx: &mut Cx| {
            let rows = vec![Row { name: "aaa" }, Row { name: "bbb" }];
            let state = TableState::new(
                rows,
                vec![
                    TableColumn::new("name", "NAME")
                        .width(ColWidth::Fixed(64.0))
                        .cell(|row: &Row| label(row.name)),
                    TableColumn::new("tag", "TAG")
                        .width(ColWidth::Fixed(64.0))
                        .cell(|row: &Row| badge(row.name, Tone::Neutral)),
                ],
            );
            cx.add(
                container().child(text("heading")).child(
                    table(state)
                        .row_height(8.0)
                        .header_height(8.0)
                        .viewport_height(64.0),
                ),
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
    use sigil::widgets::{ColWidth, TableColumn, TableState, label, table};

    #[derive(Clone)]
    struct Row;

    let grid = present(
        |cx: &mut Cx| {
            let state = TableState::new(
                vec![Row],
                vec![
                    TableColumn::new("wide", "WIDE")
                        .width(ColWidth::Fixed(32.0))
                        .cell(|_: &Row| label("aaaaaaaaaaaaaaaa")),
                    TableColumn::new("tail", "TAIL")
                        .width(ColWidth::Fixed(64.0))
                        .cell(|_: &Row| label("tail")),
                ],
            );
            cx.add(
                table(state)
                    .row_height(8.0)
                    .header_height(8.0)
                    .viewport_height(32.0),
            )
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
    let grid = present(|cx: &mut Cx| cx.add(button("Go")), 30, 8, &[]);
    assert!(grid.contains("Go"), "expected the label in:\n{grid}");
    assert!(
        !grid.contains('\u{2588}'),
        "a button holds its label, so it tints rather than filling:\n{grid}"
    );
}

#[test]
fn a_control_stands_one_line_tall() {
    let grid = present(
        |cx: &mut Cx| {
            use sigil::widgets::widget_dev::LayoutBuilderExt;
            cx.add(
                container()
                    .layout(sigil::layout::LayoutStyle::flex_column())
                    .child(button("One"))
                    .child(button("Two")),
            )
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
    use sigil::widgets::{ColWidth, TableColumn, TableState, label, table};

    #[derive(Clone)]
    struct Row {
        name: &'static str,
    }

    let grid = present(
        |cx: &mut Cx| {
            let state = TableState::new(
                vec![Row { name: "aaa" }, Row { name: "bbb" }],
                vec![
                    TableColumn::new("n", "NAME")
                        .width(ColWidth::Fixed(64.0))
                        .cell(|row: &Row| label(row.name)),
                ],
            );
            cx.add(
                table(state)
                    .row_height(8.0)
                    .header_height(8.0)
                    .viewport_fill(),
            )
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
    use std::time::Duration;

    use sigil_tui::{EventRouter, Host};

    sigil_tui::clear();
    let ticks = Rc::new(StdCell::new(0_u32));
    let counter = Rc::clone(&ticks);
    sigil::reactive::Runtime::scope("timer", || {
        let mut host = Host::cells();
        let mut router = EventRouter::new();
        host.mount(|cx: &mut Cx| cx.add(text("tick")));
        sigil_tui::every(Duration::from_millis(99), move || {
            counter.set(counter.get() + 1);
        });
        for _ in 0..9 {
            drop(step(&mut host, &mut router, [], 20, 4));
        }
    });
    sigil_tui::clear();
    assert_eq!(ticks.get(), 3, "a 99 ms timer fires 3 times in 9 frames");
}

#[test]
fn a_wide_glyph_keeps_the_next_column() {
    use unicode_width::UnicodeWidthChar;

    use sigil::widgets::{ColWidth, TableColumn, TableState, label, table};

    #[derive(Clone)]
    struct Row {
        name: &'static str,
        tail: &'static str,
    }

    let grid = present(
        |cx: &mut Cx| {
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
                        .cell(|row: &Row| label(row.name)),
                    TableColumn::new("t", "TAIL")
                        .width(ColWidth::Fixed(64.0))
                        .cell(|row: &Row| label(row.tail)),
                ],
            );
            cx.add(
                table(state)
                    .row_height(8.0)
                    .header_height(8.0)
                    .viewport_height(32.0),
            )
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
fn map_enter_is_the_shell_enter_key() {
    use sigil_tui::map_key;
    let event = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Enter,
        crossterm::event::KeyModifiers::NONE,
    );
    let mapped = map_key(&event).expect("enter maps");
    assert_eq!(mapped, key_down(Key::Enter));
}

#[test]
fn map_backtab_is_shift_tab() {
    use sigil::hal::{Key, LogicalKey, ModifiersState, ShellEvent};
    use sigil_tui::map_key;

    let event = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::BackTab,
        crossterm::event::KeyModifiers::NONE,
    );
    let Some(ShellEvent::Key(mapped)) = map_key(&event) else {
        panic!("backtab did not map");
    };
    assert_eq!(mapped.key, LogicalKey::Named(Key::Tab));
    assert_eq!(mapped.modifiers, ModifiersState::SHIFT);
}
#[test]
fn a_focus_change_reaches_the_terminal() {
    use sigil::hal::Key;
    use sigil_tui::{EventRouter, Host};

    sigil::reactive::Runtime::scope("tui-animate", || {
        let mut host = Host::cells();
        let mut router = EventRouter::new();
        host.mount(|cx: &mut Cx| {
            cx.add(
                container()
                    .child(button("One").primary())
                    .child(button("Two").primary()),
            )
        });
        host.layout((320.0, 64.0)).expect("layout");
        router.focus_next(host.cx());
        let first = step(&mut host, &mut router, [], 40, 8);
        let moved = step(&mut host, &mut router, [key_down(Key::Tab)], 40, 8);
        let settled = (0..12).fold(moved, |_, _| step(&mut host, &mut router, [], 40, 8));
        assert_ne!(
            first,
            settled,
            "moving focus must change what the terminal shows:\n{}",
            settled.text()
        );
    });
}

#[test]
fn a_still_tree_reports_no_movement() {
    use sigil_tui::Host;

    sigil::reactive::Runtime::scope("still", || {
        let mut host = Host::cells();
        host.mount(|cx: &mut Cx| cx.add(container().child(text("steady"))));
        host.layout((320.0, 64.0)).expect("layout");
        let mut nodes = Vec::new();
        assert!(
            host.repaint(&mut nodes),
            "the first paint is always a change"
        );
        host.layout((320.0, 64.0)).expect("layout");
        assert!(
            !host.repaint(&mut nodes),
            "a tree nobody touched must report no movement"
        );
    });
}

#[test]
fn a_reactive_class_change_repaints() {
    use sigil::reactive::signal;
    use sigil::style::NodeStyleExt;
    use sigil_tui::Host;

    fn fills(start: bool) -> Vec<Option<sigil::style::Rgba8>> {
        sigil::reactive::Runtime::scope("reactive-truth", || {
            let active = signal(start);
            let mut host = Host::cells();
            host.mount(move |cx: &mut Cx| {
                cx.add(
                    container()
                        .class_when(active, "bg-accent")
                        .child(text("cell")),
                )
            });
            host.layout((320.0, 64.0)).expect("layout");
            let mut nodes = Vec::new();
            host.repaint(&mut nodes);
            nodes.iter().map(|n| n.fill).collect()
        })
    }
    let cold_truth = fills(false);
    let hot_truth = fills(true);
    assert_ne!(
        cold_truth, hot_truth,
        "control: the class must resolve to a different fill at all"
    );

    sigil::reactive::Runtime::scope("reactive-class", || {
        let active = signal(false);
        let mut host = Host::cells();
        host.mount(move |cx: &mut Cx| {
            cx.add(
                container()
                    .class_when(active, "bg-accent")
                    .child(text("cell")),
            )
        });
        host.layout((320.0, 64.0)).expect("layout");
        let mut nodes = Vec::new();
        host.repaint(&mut nodes);
        let before: Vec<_> = nodes.iter().map(|n| n.fill).collect();

        active.set(true);
        host.layout((320.0, 64.0)).expect("layout");
        let moved = host.repaint(&mut nodes);
        let after: Vec<_> = nodes.iter().map(|n| n.fill).collect();

        assert_ne!(before, after, "the reactive class must change the fill");
        assert!(
            moved,
            "a changed fill must report movement, or nothing is sent"
        );
    });
}

#[test]
fn a_skipped_frame_returns_what_is_showing() {
    use sigil_tui::{EventRouter, Host, Screen};

    sigil::reactive::Runtime::scope("skip-grid", || {
        let mut host = Host::cells();
        let mut router = EventRouter::new();
        let mut screen = Screen::new(30, 6);
        host.mount(|cx: &mut Cx| cx.add(container().child(text("onscreen"))));

        let drawn = screen
            .frame(&mut host, &mut router, [], (30, 6))
            .expect("layout")
            .text();
        assert!(drawn.contains("onscreen"), "first frame paints:\n{drawn}");
        let mut sink = std::io::sink();
        screen.flush(&mut sink).expect("flush");

        let again = screen
            .frame(&mut host, &mut router, [], (30, 6))
            .expect("layout")
            .text();
        assert_eq!(drawn, again, "a skipped frame must report the screen");
    });
}

#[test]
fn only_a_change_sends_bytes() {
    use sigil::reactive::signal;
    use sigil_tui::{EventRouter, Host, Screen};

    sigil::reactive::Runtime::scope("bytes", || {
        let count = signal(0_i32);
        let label = sigil::reactive::derived(move || format!("n={}", count.get()));
        let mut host = Host::cells();
        let mut router = EventRouter::new();
        let mut screen = Screen::new(30, 6);
        host.mount(move |cx: &mut Cx| cx.add(container().child(text(label))));

        let sent = |next: &mut Screen, app: &mut Host, events: &mut EventRouter| {
            next.frame(app, events, [], (30, 6)).expect("layout");
            let mut out: Vec<u8> = Vec::new();
            next.flush(&mut out).expect("flush");
            out.len()
        };

        assert!(
            sent(&mut screen, &mut host, &mut router) > 0,
            "first frame draws"
        );
        assert_eq!(
            sent(&mut screen, &mut host, &mut router),
            0,
            "an unchanged frame sends nothing"
        );

        count.set(1);
        assert!(
            sent(&mut screen, &mut host, &mut router) > 0,
            "a changed label must reach the terminal"
        );
        assert_eq!(
            sent(&mut screen, &mut host, &mut router),
            0,
            "and then settle again"
        );
    });
}

#[test]
fn replacing_rows_repaints_the_table() {
    use sigil::widgets::{ColWidth, TableColumn, TableState, label, table};
    use sigil_tui::Host;

    #[derive(Clone)]
    struct Row {
        name: String,
    }

    sigil::reactive::Runtime::scope("replace-rows", || {
        let state = TableState::new(
            vec![Row {
                name: "before".into(),
            }],
            vec![
                TableColumn::new("n", "NAME")
                    .width(ColWidth::Fixed(96.0))
                    .cell(|row: &Row| label(row.name.clone())),
            ],
        );
        let mounted = state.clone();
        let mut host = Host::cells();
        host.mount(move |cx: &mut Cx| {
            cx.add(
                table(mounted.clone())
                    .row_height(8.0)
                    .header_height(8.0)
                    .viewport_height(64.0),
            )
        });
        host.layout((320.0, 96.0)).expect("layout");
        let mut nodes = Vec::new();
        host.repaint(&mut nodes);

        state.replace_rows(vec![Row {
            name: "after".into(),
        }]);
        host.layout((320.0, 96.0)).expect("layout");
        host.repaint(&mut nodes);
        let painted: Vec<&str> = nodes.iter().filter_map(|n| n.text.as_deref()).collect();
        assert!(
            painted.iter().any(|text| text.contains("after")),
            "replaced rows must reach the screen, painted: {painted:?}"
        );
    });
}

#[test]
fn a_click_selects_the_row_under_it() {
    use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
    use sigil::hal::Point;
    use sigil::widgets::{ColWidth, TableColumn, TableState, label, table};
    use sigil_tui::{EventRouter, Host, map_mouse};

    #[derive(Clone)]
    struct Row {
        name: &'static str,
    }

    sigil::reactive::Runtime::scope("click-row", || {
        let state = TableState::new(
            vec![
                Row { name: "first" },
                Row { name: "second" },
                Row { name: "third" },
            ],
            vec![
                TableColumn::new("n", "NAME")
                    .width(ColWidth::Fixed(96.0))
                    .cell(|row: &Row| label(row.name)),
            ],
        );
        let mounted = state.clone();
        let mut host = Host::cells();
        let mut router = EventRouter::new();
        host.mount(move |cx: &mut Cx| {
            cx.add(
                table(mounted.clone())
                    .row_height(8.0)
                    .header_height(8.0)
                    .viewport_height(64.0),
            )
        });
        drop(step(&mut host, &mut router, [], 40, 10));

        let mut at = Point::new(0.0, 0.0);
        let press = |kind| MouseEvent {
            kind,
            column: 2,
            row: 1,
            modifiers: KeyModifiers::NONE,
        };
        let events: Vec<_> = [
            MouseEventKind::Down(MouseButton::Left),
            MouseEventKind::Up(MouseButton::Left),
        ]
        .into_iter()
        .filter_map(|kind| map_mouse(&press(kind), 8.0, &mut at))
        .collect();
        drop(step(&mut host, &mut router, events, 40, 10));

        assert_eq!(
            state.selected_row().map(|row| row.name),
            Some("first"),
            "a click on the first body line must select the first row"
        );
    });
}

#[test]
fn the_wheel_scrolls_the_body() {
    use crossterm::event::{KeyModifiers, MouseEvent, MouseEventKind};
    use sigil::hal::Point;
    use sigil::widgets::{ColWidth, TableColumn, TableState, label, table};
    use sigil_tui::{EventRouter, Host, map_mouse};

    #[derive(Clone)]
    struct Row {
        name: String,
    }

    sigil::reactive::Runtime::scope("wheel", || {
        let rows: Vec<Row> = (0..60)
            .map(|i| Row {
                name: format!("row-{i:02}"),
            })
            .collect();
        let state = TableState::new(
            rows,
            vec![
                TableColumn::new("n", "NAME")
                    .width(ColWidth::Fixed(96.0))
                    .cell(|row: &Row| label(row.name.clone())),
            ],
        );
        let mounted = state;
        let mut host = Host::cells();
        let mut router = EventRouter::new();
        host.mount(move |cx: &mut Cx| {
            cx.add(
                table(mounted.clone())
                    .row_height(8.0)
                    .header_height(8.0)
                    .viewport_height(64.0),
            )
        });
        let before = step(&mut host, &mut router, [], 40, 12).text();
        assert!(before.contains("row-00"), "starts at the top:\n{before}");

        let mut at = Point::new(0.0, 0.0);
        let wheel: Vec<_> = (0..4)
            .filter_map(|_| {
                map_mouse(
                    &MouseEvent {
                        kind: MouseEventKind::ScrollDown,
                        column: 4,
                        row: 4,
                        modifiers: KeyModifiers::NONE,
                    },
                    8.0,
                    &mut at,
                )
            })
            .collect();
        let after = step(&mut host, &mut router, wheel, 40, 12).text();
        assert!(
            !after.contains("row-00"),
            "the wheel must move the window off the first row:\n{after}"
        );
    });
}

#[test]
fn a_combining_mark_keeps_its_base() {
    let marked = present(|cx: &mut Cx| cx.add(text("cafe\u{301}|")), 20, 3, &[]);
    let plain = present(|cx: &mut Cx| cx.add(text("cafe|")), 20, 3, &[]);
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

#[test]
fn work_lands_on_the_frame_clock() {
    use sigil::tasks::Status;
    use sigil_tui::{EventRouter, Host, abandon, work};

    sigil::reactive::Runtime::scope("work", || {
        abandon();
        let mut host = Host::cells();
        let mut router = EventRouter::new();
        host.mount(|cx: &mut Cx| cx.add(text("host")));

        let done = work(|| {
            std::thread::sleep(std::time::Duration::from_millis(20));
            41 + 1_i32
        });
        assert!(
            matches!(done.try_peek(), Some(Status::Pending)),
            "a job in flight reports pending"
        );

        for _ in 0..200 {
            std::thread::sleep(std::time::Duration::from_millis(2));
            drop(step(&mut host, &mut router, [], 20, 4));
            if matches!(done.try_peek(), Some(Status::Ok(_))) {
                break;
            }
        }
        assert!(
            matches!(done.try_peek(), Some(Status::Ok(42))),
            "the job's value must reach the tree"
        );
        abandon();
    });
}

#[test]
fn a_panicking_job_reports_why() {
    use sigil::tasks::Status;
    use sigil_tui::{EventRouter, Host, abandon, work};

    sigil::reactive::Runtime::scope("work-panic", || {
        abandon();
        let mut host = Host::cells();
        let mut router = EventRouter::new();
        host.mount(|cx: &mut Cx| cx.add(text("host")));

        let done = work(|| -> i32 { panic!("the disk fell over") });
        let mut reported = None;
        for _ in 0..200 {
            std::thread::sleep(std::time::Duration::from_millis(2));
            drop(step(&mut host, &mut router, [], 20, 4));
            if let Some(Status::Err(why)) = done.try_peek() {
                reported = Some(why);
                break;
            }
        }
        assert_eq!(reported.as_deref(), Some("the disk fell over"));
        abandon();
    });
}

#[test]
fn typing_reaches_a_text_field() {
    use sigil::reactive::signal;
    use sigil::widgets::text_field;
    use sigil_tui::{EventRouter, Host, map_key};

    sigil::reactive::Runtime::scope("tui-typing", || {
        let value = signal(String::new());
        let mut host = Host::cells();
        let mut router = EventRouter::new();
        host.mount(move |cx: &mut Cx| cx.add(text_field(value)));
        drop(step(&mut host, &mut router, [], 40, 6));
        router.focus_next(host.cx());
        let typed = map_key(&crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char('x'),
            crossterm::event::KeyModifiers::NONE,
        ))
        .expect("char maps");
        let grid = step(&mut host, &mut router, [typed], 40, 6);
        assert!(
            grid.text().contains('x'),
            "expected typed x in:\n{}",
            grid.text()
        );
        assert_eq!(value.peek(), "x");
    });
}

#[test]
fn a_control_chord_inserts_nothing() {
    use sigil::reactive::signal;
    use sigil::widgets::text_field;
    use sigil_tui::{EventRouter, Host, map_key};

    sigil::reactive::Runtime::scope("tui-chord", || {
        let value = signal(String::new());
        let mut host = Host::cells();
        let mut router = EventRouter::new();
        host.mount(move |cx: &mut Cx| cx.add(text_field(value)));
        drop(step(&mut host, &mut router, [], 40, 6));
        router.focus_next(host.cx());
        let chord = map_key(&crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char('x'),
            crossterm::event::KeyModifiers::CONTROL,
        ))
        .expect("char maps");
        drop(step(&mut host, &mut router, [chord], 40, 6));
        assert_eq!(value.peek(), "");
    });
}

#[test]
fn paste_inserts_whole() {
    use sigil::reactive::signal;
    use sigil::widgets::text_field;
    use sigil_tui::{EventRouter, Host, paste};

    sigil::reactive::Runtime::scope("tui-paste", || {
        let value = signal(String::new());
        let mut host = Host::cells();
        let mut router = EventRouter::new();
        host.mount(move |cx: &mut Cx| cx.add(text_field(value)));
        drop(step(&mut host, &mut router, [], 40, 6));
        router.focus_next(host.cx());
        drop(step(&mut host, &mut router, [paste("hello world")], 40, 6));
        assert_eq!(value.peek(), "hello world");
    });
}
