use super::*;
use sigil::events::EventRouter;
use sigil::kernel::Cx;

#[test]
fn long_line_does_not_grow_the_box_past_its_pane() {
    sigil::reactive::Runtime::scope("code-edit-overflow", || {
        let pane_w = 580.0;
        let lines = signal(vec!["x".repeat(200)]);
        let caret = signal((0_usize, 0_usize));
        let mut cx = Cx::new_root();
        let root = cx.add(
            container()
                .layout(LayoutStyle::block().fixed_size(pane_w, 400.0))
                .child(code_edit(lines, caret)),
        );
        let layout = layout_in(&mut cx, root, pane_w, 400.0);
        let editor = first_text_input(&cx, root);
        let result = layout.result(editor).unwrap();

        assert!(
            result.rect().width <= pane_w + 0.5,
            "code_edit box must clip to its {pane_w}px pane, not grow to the longest line; \
             got {}",
            result.rect().width,
        );
        assert!(
            result.clips(),
            "code_edit must clip overflow so a long line can't paint/hit outside its pane",
        );
    });
}

#[test]
fn long_line_run_keeps_intrinsic_width_so_paint_does_not_wrap() {
    sigil::reactive::Runtime::scope("code-edit-nowrap-run", || {
        let pane_w = 580.0;
        let lines = signal(vec!["x".repeat(200)]);
        let caret = signal((0_usize, 0_usize));
        let mut cx = Cx::new_root();
        let root = cx.add(
            container()
                .layout(LayoutStyle::block().fixed_size(pane_w, 400.0))
                .child(code_edit(lines, caret)),
        );
        let layout = layout_in(&mut cx, root, pane_w, 400.0);

        let mut widest = 0.0_f32;
        cx.visit_preorder(root, |id, _node| {
            if let Some(r) = layout.result(id) {
                widest = widest.max(r.rect().width);
            }
        });
        assert!(
            widest > pane_w + 100.0,
            "run was shrunk to the pane ({widest} <= {pane_w}) — paint will re-wrap it and \
             the long line will overlap the next row; the run must keep intrinsic width",
        );
    });
}

#[test]
fn caret_visible_when_doc_exceeds_max_rows() {
    sigil::reactive::Runtime::scope("code-edit-max-rows", || {
        let lines = signal((0..2_500).map(|i| format!("line {i}")).collect::<Vec<_>>());
        let caret = signal((2_400_usize, 0_usize));
        let mut cx = Cx::new_root();
        let root = cx.add(
            container()
                .layout(LayoutStyle::block().fixed_size(580.0, 400.0))
                .child(code_edit(lines, caret)),
        );
        let layout = layout_in(&mut cx, root, 580.0, 400.0);
        assert!(
            caret_bar_present(&cx, &layout, root),
            "caret on line 2400 of a 2500-line doc must stay in the rendered window",
        );
    });
}

#[test]
fn overflowing_rows_keep_full_height_and_do_not_compress() {
    sigil::reactive::Runtime::scope("code-edit-overflow-compress", || {
        const ROW_H: f32 = 18.0;
        let pane_h = 200.0;
        let lines = signal((0..60).map(|i| format!("line {i}")).collect::<Vec<_>>());
        let caret = signal((0_usize, 0_usize));
        let mut cx = Cx::new_root();
        let editor_node = code_edit(lines, caret).row_height(ROW_H);
        let root = cx.add(
            container()
                .layout(LayoutStyle::block().fixed_size(400.0, pane_h))
                .child(editor_node),
        );
        let layout = layout_in(&mut cx, root, 400.0, pane_h);

        let editor = first_text_input(&cx, root);
        let inner = cx.nth_child(editor, 0).expect("scroll body column");
        let count = cx.child_count(inner);
        assert!(
            count >= 12,
            "expected the overflowing rows to be laid out, got {count}"
        );
        let tops: Vec<f32> = (0..count)
            .filter_map(|i| cx.nth_child(inner, i))
            .filter_map(|row| layout.result(row).map(|r| r.rect().origin.y))
            .collect();
        for pair in tops.windows(2) {
            let step = pair[1] - pair[0];
            assert!(
                (step - ROW_H).abs() < 0.5,
                "rows compressed under overflow: step {step} != ROW_H {ROW_H} (tops={tops:?})",
            );
        }
    });
}

#[test]
fn wheel_virtualizes_and_scrolls_the_window_past_the_caret() {
    sigil::reactive::Runtime::scope("code-edit-virtualize", || {
        const ROW_H: f32 = 18.0;
        let pane_h = 200.0;
        let lines = signal((0..400).map(|i| format!("line {i}")).collect::<Vec<_>>());
        let caret = signal((0_usize, 0_usize));
        let mut cx = Cx::new_root();
        let root = cx.add(
            container()
                .layout(LayoutStyle::block().fixed_size(400.0, pane_h))
                .child(code_edit(lines, caret).row_height(ROW_H)),
        );
        let mut layout = layout_in(&mut cx, root, 400.0, pane_h);
        let editor = first_text_input(&cx, root);
        let inner = cx.nth_child(editor, 0).expect("scroll body column");

        let materialized = cx.child_count(inner);
        assert!(
            (10..150).contains(&materialized),
            "editor must render a bounded window of a 400-line file, got {materialized} rows",
        );
        assert!(caret_bar_present(&cx, &layout, root));

        let mut router = EventRouter::new();
        router.route(&mut cx, &layout, scroll_at(200.0, 100.0, -50.0 * ROW_H));
        layout
            .compute(&mut cx, root, Size::fixed(400.0, pane_h))
            .unwrap();
        assert!(
            !caret_bar_present(&cx, &layout, root),
            "a wheel-scroll must move the window past the line-0 caret (free scroll)",
        );
    });
}

#[test]
fn render_budget_tracks_the_published_viewport_height() {
    sigil::reactive::Runtime::scope("code-edit-size-reactive", || {
        const ROW_H: f32 = 18.0;
        let pane_h = 200.0;
        let lines = signal((0..400).map(|i| format!("line {i}")).collect::<Vec<_>>());
        let caret = signal((0_usize, 0_usize));
        let mut cx = Cx::new_root();
        let root = cx.add(
            container()
                .layout(LayoutStyle::block().fixed_size(400.0, pane_h))
                .child(code_edit(lines, caret).row_height(ROW_H)),
        );
        let mut layout = layout_in(&mut cx, root, 400.0, pane_h);
        for _ in 0..3 {
            sigil::app::__drive_test_frame(
                || {
                    layout
                        .compute(&mut cx, root, Size::fixed(400.0, pane_h))
                        .unwrap();
                },
                || {},
            );
        }
        let editor = first_text_input(&cx, root);
        let inner = cx.nth_child(editor, 0).expect("scroll body column");
        let materialized = cx.child_count(inner);
        assert!(
            (10..40).contains(&materialized),
            "budget must shrink to the 200px pane via size_reactive, got {materialized}"
        );
    });
}

#[test]
fn short_doc_does_not_scroll_into_blank_space() {
    sigil::reactive::Runtime::scope("code-edit-shortdoc", || {
        const ROW_H: f32 = 18.0;
        let pane_h = 200.0;
        let lines = signal((0..5).map(|i| format!("line {i}")).collect::<Vec<_>>());
        let caret = signal((0_usize, 0_usize));
        let mut cx = Cx::new_root();
        let root = cx.add(
            container()
                .layout(LayoutStyle::block().fixed_size(400.0, pane_h))
                .child(code_edit(lines, caret).row_height(ROW_H)),
        );
        let mut layout = layout_in(&mut cx, root, 400.0, pane_h);
        for _ in 0..3 {
            sigil::app::__drive_test_frame(
                || {
                    layout
                        .compute(&mut cx, root, Size::fixed(400.0, pane_h))
                        .unwrap();
                },
                || {},
            );
        }
        assert!(
            caret_bar_present(&cx, &layout, root),
            "line-0 caret shows before scroll"
        );

        let mut router = EventRouter::new();
        router.route(&mut cx, &layout, scroll_at(200.0, 100.0, -50.0 * ROW_H));
        layout
            .compute(&mut cx, root, Size::fixed(400.0, pane_h))
            .unwrap();
        assert!(
            caret_bar_present(&cx, &layout, root),
            "a doc that fits the pane must not scroll line 0 off into blank space",
        );
    });
}
