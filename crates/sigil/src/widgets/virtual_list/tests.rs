#![allow(
    clippy::float_cmp,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used,
    missing_docs
)]

use super::*;

use kernel::container;
use reactive::{Runtime, signal};

#[test]
fn build_materialises_only_visible_window() {
    Runtime::scope("sigil-virtual-list-visible-window", || {
        let mut cx = Cx::new_root();
        let items: Vec<usize> = (0..1000).collect();
        let root = cx.add(virtual_list(items, |_, _| container()));
        let content = cx.first_child(root).expect("content container");
        let visible = cx.first_child(content).expect("visible window");

        assert_eq!(cx.child_count(visible), 16);
    });
}

#[test]
fn scroll_to_end_seeds_signal_at_bottom() {
    Runtime::scope("sigil-virtual-list-scroll-to-end", || {
        // 100 rows × 30 px = 3000 px content; viewport 200 px → max scroll = 2800
        let scroll_y = reactive::signal(0.0_f32);
        let items: Vec<usize> = (0..100).collect();
        let mut cx = Cx::new_root();
        let _ = cx.add(
            virtual_list(items, |_, _| container())
                .row_height(30.0)
                .viewport_height(200.0)
                .scroll_bind(scroll_y)
                .scroll_to_end(),
        );
        assert_eq!(scroll_y.get(), 2800.0);
    });
}

#[test]
fn scroll_into_view_seeds_signal_at_row_position() {
    Runtime::scope("sigil-virtual-list-scroll-into-view", || {
        // Row 50 starts at 50 × 30 = 1500 px. Viewport is 200 px.
        // Max scroll = 100*30 - 200 = 2800. Target 1500 is within range.
        let scroll_y = reactive::signal(0.0_f32);
        let items: Vec<usize> = (0..100).collect();
        let mut cx = Cx::new_root();
        let _ = cx.add(
            virtual_list(items, |_, _| container())
                .row_height(30.0)
                .viewport_height(200.0)
                .scroll_bind(scroll_y)
                .scroll_into_view(50),
        );
        // Row 50 at top of viewport → scroll = 1500
        assert_eq!(scroll_y.get(), 1500.0);
    });
}

#[test]
fn auto_pin_to_bottom_no_op_for_static_items() {
    // Forward-compat smoke: auto_pin_to_bottom(true) must not panic.
    Runtime::scope("sigil-virtual-list-auto-pin", || {
        let scroll_y = reactive::signal(0.0_f32);
        let items: Vec<usize> = (0..10).collect();
        let mut cx = Cx::new_root();
        let _ = cx.add(
            virtual_list(items, |_, _| container())
                .row_height(30.0)
                .viewport_height(200.0)
                .scroll_bind(scroll_y)
                .auto_pin_to_bottom(true),
        );
        // No scroll seeding was requested; signal stays at 0.0
        assert_eq!(scroll_y.get(), 0.0);
    });
}

#[test]
fn viewport_fill_sets_root_dimensions_to_fill() {
    use layout::prelude::Dimension;
    Runtime::scope("sigil-virtual-list-viewport-fill", || {
        let mut cx = Cx::new_root();
        let items: Vec<usize> = (0..10).collect();
        let root = cx.add(virtual_list(items, |_, _| container()).viewport_fill());
        let style = cx
            .get(root)
            .expect("root node")
            .get_extension::<layout::LayoutStyle>()
            .expect("root has LayoutStyle")
            .clone();
        // Both axes resolve against the parent content box at layout time.
        assert_eq!(style.data().size.width, Dimension::Fill);
        assert_eq!(style.data().size.height, Dimension::Fill);
    });
}

#[test]
fn viewport_fill_tracks_parent_pane_measured_size() {
    // Drop a virtual_list with .viewport_fill() into a fixed-size parent
    // and verify the rendered viewport rect tracks the parent's content box.
    use hal::{LogicalSize, Size};
    use kernel::TextContent;
    use layout::prelude::LayoutBuilderExt;
    use layout::{LayoutEngine, LayoutStyle, TextMeasure};

    struct StubMeasure;
    impl TextMeasure for StubMeasure {
        fn measure(&self, _content: &TextContent, _space: Size) -> LogicalSize {
            LogicalSize::new(0.0, 0.0)
        }
    }

    Runtime::scope("sigil-virtual-list-viewport-fill-layout", || {
        let mut cx = Cx::new_root();
        // Parent container has a fixed 400×300 content box.
        let items: Vec<usize> = (0..50).collect();
        let parent = cx.add(
            container()
                .layout(LayoutStyle::block().fixed_size(400.0, 300.0))
                .child(
                    virtual_list(items, |_, _| container())
                        .row_height(24.0)
                        .viewport_fill(),
                ),
        );
        let viewport = cx.first_child(parent).expect("virtual_list root");

        let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
        layout
            .compute(&mut cx, parent, Size::fixed(800.0, 600.0))
            .expect("layout");

        let parent_rect = layout.result(parent).expect("parent layout").rect();
        let viewport_rect = layout.result(viewport).expect("viewport layout").rect();
        assert!(
            (viewport_rect.width - parent_rect.width).abs() < 0.5,
            "viewport width {} should track parent width {}",
            viewport_rect.width,
            parent_rect.width
        );
        assert!(
            (viewport_rect.height - parent_rect.height).abs() < 0.5,
            "viewport height {} should track parent height {}",
            viewport_rect.height,
            parent_rect.height
        );
    });
}

#[test]
fn select_fires_with_selected_index() {
    use std::cell::Cell;
    use std::rc::Rc;
    Runtime::scope("sigil-virtual-list-select", || {
        let last_index: Rc<Cell<Option<usize>>> = Rc::new(Cell::new(None));
        let last_clone = Rc::clone(&last_index);

        let items: Vec<usize> = (0..20).collect();
        let mut builder = virtual_list(items, |_, _| container())
            .row_height(30.0)
            .viewport_height(200.0)
            .selection(SelectionMode::Single);

        // Capture the selection signal before wiring .select().
        let selection = builder.selection_state();
        let builder = builder.select(move |i| {
            last_clone.set(Some(i));
        });

        let mut cx = Cx::new_root();
        let _ = cx.add(builder);

        // Trigger a selection change — the effect re-runs.
        selection.update(|state| state.click(7));
        assert_eq!(last_index.get(), Some(7));
    });
}

// ── items_bind / reactive path ────────────────────────────────────────────────

/// Helper: navigate root → content → visible window for `build_bound` trees.
///
/// Tree shape produced by `build_bound`:
///   root (`viewport_container`)
///   └─ content (`content_container`)
///      └─ visible (`children_reactive` container)
fn bound_visible(cx: &Cx, root: NodeId) -> NodeId {
    let content = cx.first_child(root).expect("content child of root");
    cx.first_child(content).expect("visible child of content")
}

#[test]
fn items_bind_initial_signal_value_builds_rows() {
    Runtime::scope("sigil-vlist-items-bind-initial", || {
        let items = signal(vec!["a", "b", "c"]);
        let mut cx = Cx::new_root();
        let root = cx.add(virtual_list(Vec::<&str>::new(), |_, _| container()).items_bind(items));
        cx.flush_reactive_children();
        let visible = bound_visible(&cx, root);
        assert_eq!(
            cx.child_count(visible),
            3,
            "3 rows for initial 3-item signal"
        );
    });
}

#[test]
fn items_bind_signal_mutation_replaces_rows() {
    Runtime::scope("sigil-vlist-items-bind-mutate", || {
        let items = signal(vec!["a", "b", "c"]);
        let mut cx = Cx::new_root();
        let root = cx.add(virtual_list(Vec::<&str>::new(), |_, _| container()).items_bind(items));
        cx.flush_reactive_children();
        assert_eq!(
            cx.child_count(bound_visible(&cx, root)),
            3,
            "3 rows before mutation"
        );

        items.set(vec!["x", "y"]);
        cx.flush_reactive_children();
        assert_eq!(
            cx.child_count(bound_visible(&cx, root)),
            2,
            "2 rows after mutation to 2-item Vec"
        );
    });
}

#[test]
fn items_bind_static_path_still_works() {
    // Guard: existing `virtual_list(vec, render)` without `.items_bind()` must
    // still build correctly (smoke test for the bound/static split regression).
    Runtime::scope("sigil-vlist-items-bind-static-smoke", || {
        let items: Vec<usize> = (0..5).collect();
        let mut cx = Cx::new_root();
        let root = cx.add(virtual_list(items, |_, _| container()));
        let content = cx.first_child(root).expect("content");
        let visible = cx.first_child(content).expect("visible window");
        // Static path materialises the visible window inline — no flush needed.
        // With DEFAULT_VIEWPORT_HEIGHT 480 and DEFAULT_ROW_HEIGHT 32, a 5-item
        // list fits entirely in the first window (5 < 16 = ceil(480/32)+1).
        assert_eq!(
            cx.child_count(visible),
            5,
            "all 5 rows visible in static path"
        );
    });
}

#[test]
fn items_bind_empty_to_full_to_empty() {
    Runtime::scope("sigil-vlist-items-bind-empty-cycle", || {
        let items = signal(Vec::<&str>::new());
        let mut cx = Cx::new_root();
        let root = cx.add(virtual_list(Vec::<&str>::new(), |_, _| container()).items_bind(items));
        cx.flush_reactive_children();
        assert_eq!(
            cx.child_count(bound_visible(&cx, root)),
            0,
            "0 rows for empty initial signal"
        );

        items.set(vec!["p", "q", "r"]);
        cx.flush_reactive_children();
        assert_eq!(
            cx.child_count(bound_visible(&cx, root)),
            3,
            "3 rows after set to 3-item Vec"
        );

        items.set(Vec::new());
        cx.flush_reactive_children();
        assert_eq!(
            cx.child_count(bound_visible(&cx, root)),
            0,
            "0 rows after clearing Vec"
        );
    });
}
