#![allow(
    clippy::float_cmp,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used,
    missing_docs
)]

use super::*;

use sigil_kernel::{Cx, NodeId, container};
use sigil_reactive::{Runtime, signal};

use super::Identifiable;

#[test]
fn build_materialises_only_visible_window() {
    Runtime::scope("sigil-virtual-list-visible-window", || {
        let mut cx = Cx::new_root();
        let items: Vec<usize> = (0..1000).collect();
        let root = cx.add(virtual_list(items, |_, _| container()));
        let content = cx.first_child(root).expect("content container");
        let visible = cx.first_child(content).expect("visible window");

        assert_eq!(cx.child_count(visible), 30);
    });
}

#[test]
fn overscan_zero_mounts_the_geometric_page() {
    Runtime::scope("sigil-virtual-list-overscan-zero", || {
        let mut cx = Cx::new_root();
        let items: Vec<usize> = (0..1000).collect();
        let root = cx.add(virtual_list(items, |_, _| container()).overscan(0));
        let content = cx.first_child(root).expect("content container");
        let visible = cx.first_child(content).expect("visible window");

        assert_eq!(cx.child_count(visible), 15);
    });
}

#[test]
fn overscan_zero_bind_mounts_the_geometric_page() {
    Runtime::scope("sigil-virtual-list-overscan-zero-bind", || {
        let items = signal((0..1000).collect::<Vec<usize>>());
        let mut cx = Cx::new_root();
        let root = cx.add(
            virtual_list_bind(items, |_, _| container())
                .overscan(0)
                .row_height(32.0)
                .viewport_height(480.0),
        );
        cx.flush_reactive_children();
        assert_eq!(cx.child_count(bound_visible(&cx, root)), 15);
    });
}

#[test]
fn scroll_to_end_seeds_signal_at_bottom() {
    Runtime::scope("sigil-virtual-list-scroll-to-end", || {
        // 100 rows × 30 px = 3000 px content; viewport 200 px → max scroll = 2800
        let scroll_y = sigil_reactive::signal(0.0_f32);
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
        let scroll_y = sigil_reactive::signal(0.0_f32);
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
    Runtime::scope("sigil-virtual-list-auto-pin", || {
        let scroll_y = sigil_reactive::signal(0.0_f32);
        let items: Vec<usize> = (0..10).collect();
        let mut cx = Cx::new_root();
        let _ = cx.add(
            virtual_list(items, |_, _| container())
                .row_height(30.0)
                .viewport_height(200.0)
                .scroll_bind(scroll_y)
                .auto_pin_to_bottom(true),
        );
        assert_eq!(scroll_y.get(), 0.0);
    });
}

#[test]
fn bound_scroll_to_end_seeds_from_live_count() {
    Runtime::scope("sigil-vlist-bind-scroll-to-end", || {
        let scroll_y = signal(0.0_f32);
        let items = signal((0..100_u32).collect::<Vec<_>>());
        let mut cx = Cx::new_root();
        let _ = cx.add(
            virtual_list_bind(items, |_, _| container())
                .row_height(30.0)
                .viewport_height(200.0)
                .scroll_bind(scroll_y)
                .scroll_to_end(),
        );
        assert_eq!(scroll_y.get(), 2800.0);
    });
}

#[test]
fn auto_pin_follows_grow_when_already_at_bottom() {
    Runtime::scope("sigil-vlist-auto-pin-follow", || {
        let scroll_y = signal(0.0_f32);
        let items = signal((0..100_u32).collect::<Vec<_>>());
        let mut cx = Cx::new_root();
        let _ = cx.add(
            virtual_list_bind(items, |_, _| container())
                .row_height(30.0)
                .viewport_height(200.0)
                .scroll_bind(scroll_y)
                .scroll_to_end()
                .auto_pin_to_bottom(true),
        );
        assert_eq!(scroll_y.get(), 2800.0);
        items.set((0..120_u32).collect());
        assert_eq!(scroll_y.get(), 3400.0);
    });
}

#[test]
fn auto_pin_leaves_scroll_when_user_scrolled_up() {
    Runtime::scope("sigil-vlist-auto-pin-hold", || {
        let scroll_y = signal(0.0_f32);
        let items = signal((0..100_u32).collect::<Vec<_>>());
        let mut cx = Cx::new_root();
        let _ = cx.add(
            virtual_list_bind(items, |_, _| container())
                .row_height(30.0)
                .viewport_height(200.0)
                .scroll_bind(scroll_y)
                .auto_pin_to_bottom(true),
        );
        assert_eq!(scroll_y.get(), 0.0);
        items.set((0..120_u32).collect());
        assert_eq!(scroll_y.get(), 0.0);
    });
}

#[test]
fn viewport_fill_sets_root_dimensions_to_fill() {
    use sigil_layout::prelude::Dimension;
    Runtime::scope("sigil-virtual-list-viewport-fill", || {
        let mut cx = Cx::new_root();
        let items: Vec<usize> = (0..10).collect();
        let root = cx.add(virtual_list(items, |_, _| container()).viewport_fill());
        let style = cx
            .get(root)
            .expect("root node")
            .get_extension::<sigil_layout::LayoutStyle>()
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
    use sigil_hal::{LogicalSize, Size};
    use sigil_kernel::TextContent;
    use sigil_layout::prelude::LayoutBuilderExt;
    use sigil_layout::{LayoutEngine, LayoutStyle, TextMeasure};

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

/// Helper: navigate root → reactive host → content → visible window for
/// `build_bound` trees.
///
/// Tree shape produced by `build_bound` (windowed, 5/5 Wave 2 T6-prereq):
///   root (`viewport_container`)
///   └─ host (block)
///      └─ content (`content_container`)
///         └─ visible window (absolute-positioned rows container)
fn bound_visible(cx: &Cx, root: NodeId) -> NodeId {
    let host = cx
        .first_child(root)
        .expect("children_reactive host child of root");
    let content = cx.first_child(host).expect("content child of host");
    cx.first_child(content)
        .expect("visible window child of content")
}

#[test]
fn items_bind_initial_signal_value_builds_rows() {
    Runtime::scope("sigil-vlist-items-bind-initial", || {
        let items = signal(vec!["a", "b", "c"]);
        let mut cx = Cx::new_root();
        let root = cx.add(virtual_list_bind(items, |_, _| container()));
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
        let root = cx.add(virtual_list_bind(items, |_, _| container()));
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
    // Guard: static `virtual_list(vec, render)` without `virtual_list_bind` must
    // still build correctly (smoke test for the bound/static split regression).
    Runtime::scope("sigil-vlist-items-bind-static-smoke", || {
        let items: Vec<usize> = (0..5).collect();
        let mut cx = Cx::new_root();
        let root = cx.add(virtual_list(items, |_, _| container()));
        let content = cx.first_child(root).expect("content");
        let visible = cx.first_child(content).expect("visible window");
        // Static path materialises the visible window inline — no flush needed.
        // With DEFAULT_VIEWPORT_HEIGHT 480 and DEFAULT_ROW_HEIGHT 32, a 5-item
        // list fits entirely in the first window (5 < 30 = page + overscan).
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
        let root = cx.add(virtual_list_bind(items, |_, _| container()));
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

#[test]
fn items_bind_window_advances_in_place_on_scroll() {
    Runtime::scope("sigil-vlist-bind-scroll-advance", || {
        let items = signal((0..10_000).collect::<Vec<usize>>());
        let scroll_y = signal(0.0_f32);
        let mut cx = Cx::new_root();
        let root = cx.add(
            virtual_list_bind(items, |index, _| container().label(format!("row {index}")))
                .scroll_bind(scroll_y)
                .row_height(20.0)
                .viewport_height(100.0),
        );
        cx.flush_reactive_children();
        // 5 in view + 5 overscan below at scroll 0.
        assert_eq!(cx.child_count(bound_visible(&cx, root)), 10);
        assert_eq!(first_row_label(&cx, root), Some("row 0".to_string()));

        // Scroll to row 200 WITHOUT touching the items signal — the same
        // tree must re-render the window in place (no rebuild).
        scroll_y.set(4_000.0);
        cx.flush_reactive_children();
        assert_eq!(cx.child_count(bound_visible(&cx, root)), 15);
        assert_eq!(
            first_row_label(&cx, root),
            Some("row 195".to_string()),
            "scroll_y write must advance the padded window in place"
        );
    });
}

#[test]
fn scroll_one_row_keeps_overlapping_node() {
    Runtime::scope("sigil-vlist-keyed-reuse", || {
        let items = signal((0..1_000u32).collect::<Vec<_>>());
        let scroll_y = signal(0.0_f32);
        let mut cx = Cx::new_root();
        let root = cx.add(
            virtual_list_bind(items, |index, _| container().label(format!("row {index}")))
                .scroll_bind(scroll_y)
                .row_height(20.0)
                .viewport_height(100.0),
        );
        cx.flush_reactive_children();
        let before = bound_visible(&cx, root);
        let keep = cx.nth_child(before, 1).expect("row 1");
        scroll_y.set(20.0);
        cx.flush_reactive_children();
        let after = bound_visible(&cx, root);
        let still = cx.nth_child(after, 1).expect("row 1 still mounted");
        assert_eq!(still, keep, "overlapping row must keep NodeId");
        assert_eq!(first_row_label(&cx, root), Some("row 0".to_string()));
    });
}

#[test]
fn identified_reorder_keeps_row_nodes() {
    #[derive(Clone, Copy, PartialEq)]
    struct Row {
        id: u8,
        name: &'static str,
    }
    impl Identifiable for Row {
        type Id = u8;
        fn id(&self) -> Self::Id {
            self.id
        }
    }

    Runtime::scope("sigil-vlist-identified-reorder", || {
        let items = signal(vec![Row { id: 1, name: "a" }, Row { id: 2, name: "b" }]);
        let mut cx = Cx::new_root();
        let root = cx.add(virtual_list(items, |_, row| container().label(row.name)).identified());
        cx.flush_reactive_children();
        let visible = bound_visible(&cx, root);
        let before = [
            cx.nth_child(visible, 0).expect("row a"),
            cx.nth_child(visible, 1).expect("row b"),
        ];

        items.set(vec![Row { id: 2, name: "b" }, Row { id: 1, name: "a" }]);
        cx.flush_reactive_children();
        let reordered = bound_visible(&cx, root);
        let after = [
            cx.nth_child(reordered, 0).expect("row b"),
            cx.nth_child(reordered, 1).expect("row a"),
        ];
        assert_eq!(after, [before[1], before[0]]);
        assert_eq!(first_row_label(&cx, root), Some("b".to_string()));
    });
}

#[test]
fn items_bind_same_len_keeps_nodes() {
    Runtime::scope("sigil-vlist-same-len", || {
        let items = signal(vec!["a", "b", "c"]);
        let mut cx = Cx::new_root();
        let root = cx.add(virtual_list_bind(items, |_, _| container()));
        cx.flush_reactive_children();
        let keep = cx.first_child(bound_visible(&cx, root)).expect("row 0");
        items.set(vec!["x", "y", "z"]);
        cx.flush_reactive_children();
        let first = cx.first_child(bound_visible(&cx, root)).expect("row 0");
        assert_eq!(first, keep, "same length must keep NodeId");
    });
}
/// Accessible name of the first visible row.
fn first_row_label(cx: &Cx, root: NodeId) -> Option<String> {
    let visible = bound_visible(cx, root);
    let first = cx.first_child(visible)?;
    name_of(cx, first).or_else(|| cx.first_child(first).and_then(|row| name_of(cx, row)))
}

fn name_of(cx: &Cx, id: NodeId) -> Option<String> {
    match &cx.get(id)?.a11y.name {
        sigil_kernel::AccessibleName::Static(name) => Some(name.clone()),
        _ => None,
    }
}
