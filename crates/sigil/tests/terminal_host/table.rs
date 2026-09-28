use super::*;

#[test]
fn replacing_rows_repaints_the_table() {
    use sigil::Host;
    use sigil::widgets::{ColWidth, TableColumn, TableState, table};

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
                    .cell(|row: &Row| text(row.name.clone()).nowrap()),
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
        drop(host.emit((320.0, 96.0)));

        state.replace_rows(vec![Row {
            name: "after".into(),
        }]);
        host.layout((320.0, 96.0)).expect("layout");
        let scene = host.emit((320.0, 96.0));
        let painted: Vec<&str> = scene
            .commands
            .iter()
            .filter_map(|command| match command {
                sigil::scene::SceneCommand::GlyphRun(draw) => Some(draw.run.text.as_str()),
                _ => None,
            })
            .collect();
        assert!(
            painted.iter().any(|text| text.contains("after")),
            "replaced rows must reach the screen, painted: {painted:?}"
        );
    });
}

#[test]
fn a_click_selects_the_row_under_it() {
    use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
    use sigil::Host;
    use sigil::events::EventRouter;
    use sigil::hal::Point;
    use sigil::terminal::map_mouse;
    use sigil::widgets::{ColWidth, TableColumn, TableState, table};

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
                    .cell(|row: &Row| text(row.name).nowrap()),
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
        let grid = step(&mut host, &mut router, [], 40, 10);
        let text = grid.text();
        let (line, col) = text
            .lines()
            .enumerate()
            .find_map(|(index, painted)| {
                painted
                    .find("first")
                    .map(|col| (index, painted[..col].chars().count()))
            })
            .expect("the first row paints its label");
        let mut at = Point::new(0.0, 0.0);
        let press = |kind| MouseEvent {
            kind,
            column: u16::try_from(col).expect("column fits"),
            row: u16::try_from(line).expect("row fits"),
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
    use sigil::Host;
    use sigil::events::EventRouter;
    use sigil::hal::Point;
    use sigil::terminal::map_mouse;
    use sigil::widgets::{ColWidth, TableColumn, TableState, table};

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
                    .cell(|row: &Row| text(row.name.clone()).nowrap()),
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
