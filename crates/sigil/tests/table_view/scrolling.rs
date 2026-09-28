use super::*;

#[test]
fn quick_filter_keeps_keyboard_row_count_live() {
    sigil::reactive::Runtime::scope("table-view-live-count", || {
        let rows = (0..30_u32)
            .map(|i| {
                item(
                    &format!("{}-{i:02}", if i % 3 == 0 { "keep" } else { "drop" }),
                    i64::from(i),
                )
            })
            .collect::<Vec<_>>();
        let state = TableState::new(
            rows,
            vec![
                TableColumn::new("name", "Name")
                    .sort_by(|row: &Item| row.name.clone())
                    .filterable(|row: &Item, query: &str| row.name.contains(query))
                    .width(ColWidth::Fixed(WIDTH))
                    .cell(|row: &Item| container().child(text(row.name.clone()))),
            ],
        );
        let mut cx = Cx::new_root();
        let root = cx.add(
            table(state.clone())
                .row_height(ROW_H)
                .header_height(HEADER_H)
                .viewport_width(WIDTH)
                .viewport_height(100.0),
        );
        let _layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
        state.quick_filter().set("keep".to_owned());
        let layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
        assert_eq!(state.visible().get().len(), 10);

        let body = cx
            .get(root)
            .and_then(|node| node.children().get(1).copied())
            .expect("grid root has a body child");
        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(body));
        router.route(&mut cx, &layout, key_down(Key::End, PhysicalKey::End));
        let _layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
        let texts = glyph_texts(&cx, root);
        assert!(
            texts.iter().any(|text| text == "keep-27"),
            "End must land on the last FILTERED row; painted: {texts:?}",
        );
    });
}

#[test]
fn wheel_scroll_advances_the_window() {
    sigil::reactive::Runtime::scope("table-view-wheel", || {
        let rows = (0..100_u32)
            .map(|i| item(&format!("r{i:03}"), i64::from(i)))
            .collect::<Vec<_>>();
        let state = TableState::new(
            rows,
            vec![
                TableColumn::new("name", "Name")
                    .sort_by(|row: &Item| row.name.clone())
                    .width(ColWidth::Fixed(WIDTH))
                    .cell(|row: &Item| container().child(text(row.name.clone()))),
            ],
        );
        let mut cx = Cx::new_root();
        let root = cx.add(
            table(state)
                .row_height(ROW_H)
                .header_height(HEADER_H)
                .viewport_width(WIDTH)
                .viewport_height(100.0),
        );
        let layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
        let first_body_text = |cx: &Cx| {
            glyph_texts(cx, root)
                .into_iter()
                .find(|text| text.starts_with('r'))
        };
        assert_eq!(first_body_text(&cx).as_deref(), Some("r000"));

        let mut router = EventRouter::new();
        router.route(
            &mut cx,
            &layout,
            ShellEvent::Scroll(ScrollEvent::new(ScrollEventInit {
                position: Point::new(50.0, 100.0),
                delta: ScrollDelta::Pixels(Point::new(0.0, -120.0)),
                modifiers: ModifiersState::empty(),
                phase: ScrollPhase::Moved,
            })),
        );
        let _layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
        let texts = glyph_texts(&cx, root);
        let body = texts
            .iter()
            .filter(|text| text.starts_with('r'))
            .collect::<Vec<_>>();
        assert_eq!(
            body.first().map(|s| s.as_str()),
            Some("r005"),
            "wheel delta advances the visible window: {texts:?}",
        );
        assert_eq!(
            body.get(1).map(|s| s.as_str()),
            Some("r006"),
            "first fully visible row is six rows down: {texts:?}",
        );
    });
}

#[test]
fn empty_filter_shows_no_rows() {
    sigil::reactive::Runtime::scope("table-view-empty", || {
        let state = demo_state();
        state.quick_filter().set("zzzz".into());
        let mut cx = Cx::new_root();
        let root = cx.add(
            table(state)
                .row_height(ROW_H)
                .header_height(HEADER_H)
                .viewport_width(WIDTH)
                .viewport_height(VIEW_H),
        );
        let _ = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
        let texts = glyph_texts(&cx, root);
        assert!(
            texts.iter().any(|text| text.contains("No rows")),
            "empty table shows the default empty copy: {texts:?}"
        );
    });
}

#[test]
fn grid_a11y_channels_attach() {
    sigil::reactive::Runtime::scope("table-view-a11y", || {
        let state = demo_state();
        let mut cx = Cx::new_root();
        let root = cx.add(
            table(state)
                .row_height(ROW_H)
                .header_height(HEADER_H)
                .viewport_width(WIDTH)
                .viewport_height(VIEW_H),
        );
        let _layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
        let grid = cx.get(root).expect("root node");
        assert_eq!(grid.a11y.role, Role::Grid);
        assert_eq!(grid.a11y.column_count, Some(2));
        assert_eq!(
            grid.a11y.row_count.map(|count| count.get()),
            Some(5),
            "four body rows + one header row",
        );

        let header = grid.children().first().copied().expect("header row");
        let header = cx.get(header).expect("header node");
        assert_eq!(header.a11y.role, Role::Row);
        assert_eq!(header.a11y.row_index, Some(1));
        let track = header
            .children()
            .first()
            .copied()
            .expect("header cell track");
        let header_cell = cx
            .get(track)
            .expect("track node")
            .children()
            .first()
            .copied()
            .expect("header cell");
        let header_cell = cx.get(header_cell).expect("header cell node");
        assert_eq!(header_cell.a11y.role, Role::ColumnHeader);
        assert_eq!(header_cell.a11y.column_index, Some(1));
        assert!(
            header_cell.a11y.sort_direction.is_some(),
            "sortable header carries the sort-direction channel",
        );
    });
}
