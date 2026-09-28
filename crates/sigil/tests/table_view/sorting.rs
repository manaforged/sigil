use super::*;

#[test]
fn header_click_cycles_sort_and_reorders_rows() {
    sigil::reactive::Runtime::scope("table-view-sort-cycle", || {
        let state = demo_state();
        let mut cx = Cx::new_root();
        let root = cx.add(
            table(state.clone())
                .row_height(ROW_H)
                .header_height(HEADER_H)
                .viewport_width(WIDTH)
                .viewport_height(VIEW_H),
        );
        let layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
        let mut router = EventRouter::new();

        click(&mut router, &mut cx, &layout, 50.0, 14.0);
        assert_eq!(
            state.sort_key().get(),
            Some(SortKey {
                column: "name",
                dir: SortDir::Ascending,
            }),
            "first click sorts ascending",
        );
        assert_eq!(visible_names(&state), ["alpha", "beta", "delta", "gamma"]);

        click(&mut router, &mut cx, &layout, 50.0, 14.0);
        assert_eq!(
            state.sort_key().get(),
            Some(SortKey {
                column: "name",
                dir: SortDir::Descending,
            })
        );
        assert_eq!(visible_names(&state), ["gamma", "delta", "beta", "alpha"]);

        click(&mut router, &mut cx, &layout, 50.0, 14.0);
        assert_eq!(state.sort_key().get(), None, "third click clears the sort");
        assert_eq!(visible_names(&state), ["gamma", "alpha", "delta", "beta"]);

        click(&mut router, &mut cx, &layout, 150.0, 14.0);
        assert_eq!(
            state.sort_key().get(),
            Some(SortKey {
                column: "count",
                dir: SortDir::Ascending,
            })
        );
    });
}

#[test]
fn row_click_drives_selection_with_modifiers() {
    sigil::reactive::Runtime::scope("table-view-selection", || {
        let state = demo_state();
        let mut cx = Cx::new_root();
        let root = cx.add(
            table(state.clone())
                .row_height(ROW_H)
                .header_height(HEADER_H)
                .viewport_width(WIDTH)
                .viewport_height(VIEW_H),
        );
        let layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
        let mut router = EventRouter::new();

        let row_y = |n: usize| {
            #[allow(clippy::cast_precision_loss)]
            let n = n as f32;
            (n + 0.5).mul_add(ROW_H, HEADER_H)
        };
        router.route(
            &mut cx,
            &layout,
            pointer(PointerPhase::Down, 50.0, row_y(1), ModifiersState::empty()),
        );
        assert!(state.selection().get().contains(&1), "plain click selects");

        router.route(
            &mut cx,
            &layout,
            pointer(PointerPhase::Down, 50.0, row_y(3), ModifiersState::CONTROL),
        );
        let selected = state.selection().get();
        assert!(selected.contains(&1) && selected.contains(&3), "ctrl adds");

        router.route(
            &mut cx,
            &layout,
            pointer(PointerPhase::Down, 50.0, row_y(2), ModifiersState::SHIFT),
        );
        let selected = state.selection().get();
        assert!(selected.contains(&2) && selected.contains(&3) && !selected.contains(&1));
    });
}

#[test]
fn selected_row_paints_selection_bg() {
    sigil::reactive::Runtime::scope("table-view-selected-pixel", || {
        let state = demo_state();
        let mut cx = Cx::new_root();
        let root = cx.add(
            table(state.clone())
                .row_height(ROW_H)
                .header_height(HEADER_H)
                .viewport_width(WIDTH)
                .viewport_height(VIEW_H),
        );
        let _layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
        let selection_bg = Theme::default().accent_scale.step(5);
        assert!(
            !rect_colors(&cx, root).contains(&selection_bg),
            "nothing selected at rest",
        );
        state.select_only(1);
        assert!(
            rect_colors(&cx, root).contains(&selection_bg),
            "selected row paints the accent-5 selection slice; \
             colours: {:?}",
            rect_colors(&cx, root),
        );
        state.clear_selection();
        assert!(
            !rect_colors(&cx, root).contains(&selection_bg),
            "clearing the selection restores the rest paint",
        );
    });
}

#[test]
fn hover_row_paints_hover_slice() {
    sigil::reactive::Runtime::scope("table-view-hover-pixel", || {
        let state = demo_state();
        let mut cx = Cx::new_root();
        let root = cx.add(
            table(state)
                .row_height(ROW_H)
                .header_height(HEADER_H)
                .viewport_width(WIDTH)
                .viewport_height(VIEW_H),
        );
        let layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
        let mut router = EventRouter::new();
        let hover = Theme::default().surface_hover;
        assert_eq!(
            rect_colors(&cx, root)
                .iter()
                .filter(|c| **c == hover)
                .count(),
            0,
            "rest: header is surface, no hover tint",
        );
        router.route(
            &mut cx,
            &layout,
            pointer(
                PointerPhase::Move,
                50.0,
                HEADER_H + ROW_H / 2.0,
                ModifiersState::empty(),
            ),
        );
        assert_eq!(
            rect_colors(&cx, root)
                .iter()
                .filter(|c| **c == hover)
                .count(),
            1,
            "hovered row paints a surface-hover slice",
        );
        router.route(
            &mut cx,
            &layout,
            pointer(PointerPhase::Move, 500.0, 400.0, ModifiersState::empty()),
        );
        assert_eq!(
            rect_colors(&cx, root)
                .iter()
                .filter(|c| **c == hover)
                .count(),
            0,
            "pointer leave restores the rest paint",
        );
    });
}

#[test]
fn hundred_thousand_rows_window_and_sort_without_rebuild() {
    sigil::reactive::Runtime::scope("table-view-100k", || {
        let rows = (0..100_000_u32)
            .map(|i| item(&format!("r{:07}", 99_999 - i), i64::from(i)))
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
            table(state.clone())
                .row_height(ROW_H)
                .header_height(HEADER_H)
                .viewport_width(WIDTH)
                .viewport_height(VIEW_H),
        );
        let _layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
        let body_runs = |cx: &Cx| {
            glyph_texts(cx, root)
                .into_iter()
                .filter(|text| text.starts_with('r'))
                .collect::<Vec<_>>()
        };
        assert_eq!(body_runs(&cx).len(), 11, "window stays viewport-sized");
        assert_eq!(body_runs(&cx).first().map(String::as_str), Some("r0099999"));

        state.toggle_sort("name");
        let _layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
        assert_eq!(body_runs(&cx).len(), 11, "window cost unchanged by sort");
        assert_eq!(body_runs(&cx).first().map(String::as_str), Some("r0000000"));
    });
}
