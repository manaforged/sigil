//! Phase 2.4 acceptance — splitter widget structural + layout tests.
//!
//! Coverage:
//! - Horizontal splitter with 3 panes lays each pane to its initial size.
//! - Vertical splitter does the same on the y axis.
//! - Nested splitter (vertical → horizontal) builds without panic and
//!   surfaces all leaf panes in the tree.
//! - `sizes_bind` populates the bound signal with the initial sizes.
//!
//! Drag-driven resize testing is deferred — the captured-pointer flow
//! needs viewport-aware event synthesis at specific handle positions.
//! Tracked in plan followups.

#![allow(missing_docs)]
#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use arboard as _;
use criterion as _;
use image as _;
use pollster as _;
use sigil_actions as _;
use sigil_animation as _;
use sigil_animation_runtime as _;
use sigil_app as _;
use sigil_bidi as _;
use sigil_events as _;
use sigil_hal as _;
use sigil_kernel as _;
use sigil_layout as _;
use sigil_profile as _;
use sigil_reactive as _;
use sigil_renderer as _;
use sigil_style as _;
use sigil_tasks as _;
#[cfg(feature = "tui")]
use sigil_tui as _;
use sigil_widgets as _;
use wgpu as _;

use sigil::prelude::*;
use sigil::widgets::{SplitterOrientation, splitter};

struct StubMeasure;

impl TextMeasure for StubMeasure {
    fn measure(&self, _content: &TextContent, _space: sigil_hal::Size) -> sigil_hal::LogicalSize {
        sigil_hal::LogicalSize::new(0.0, 0.0)
    }
}

fn compute(cx: &mut Cx, root: NodeId, width: f32, height: f32) -> LayoutEngine {
    let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
    layout
        .compute(cx, root, Size::fixed(width, height))
        .expect("layout");
    layout
}

#[test]
fn horizontal_splitter_lays_out_panes_with_initial_sizes() {
    let mut cx = Cx::new_root();
    let root = cx.add(
        splitter(SplitterOrientation::Horizontal)
            .child(container().label("a"), 200.0)
            .child(container().label("b"), 200.0)
            .child(container().label("c"), 200.0),
    );

    let layout = compute(&mut cx, root, 612.0, 200.0);

    // 3 panes + 2 handles interleaved: pane / handle / pane / handle / pane.
    let pane_0 = cx.nth_child(root, 0).expect("pane 0");
    let pane_1 = cx.nth_child(root, 2).expect("pane 1");
    let pane_2 = cx.nth_child(root, 4).expect("pane 2");

    let w0 = layout.result(pane_0).expect("pane 0 layout").rect().width;
    let w1 = layout.result(pane_1).expect("pane 1 layout").rect().width;
    let w2 = layout.result(pane_2).expect("pane 2 layout").rect().width;

    assert!(
        (w0 - 200.0).abs() < 0.5,
        "pane 0 expected ~200 px wide, got {w0}",
    );
    assert!(
        (w1 - 200.0).abs() < 0.5,
        "pane 1 expected ~200 px wide, got {w1}",
    );
    assert!(
        (w2 - 200.0).abs() < 0.5,
        "pane 2 expected ~200 px wide, got {w2}",
    );
}

#[test]
fn vertical_splitter_lays_out_panes_with_initial_sizes() {
    let mut cx = Cx::new_root();
    let root = cx.add(
        splitter(SplitterOrientation::Vertical)
            .child(container().label("top"), 100.0)
            .child(container().label("bottom"), 100.0),
    );

    let layout = compute(&mut cx, root, 300.0, 206.0);

    let pane_top = cx.nth_child(root, 0).expect("top pane");
    let pane_bottom = cx.nth_child(root, 2).expect("bottom pane");

    let h_top = layout.result(pane_top).expect("top layout").rect().height;
    let h_bottom = layout
        .result(pane_bottom)
        .expect("bottom layout")
        .rect()
        .height;

    assert!(
        (h_top - 100.0).abs() < 0.5,
        "top expected ~100 px tall, got {h_top}",
    );
    assert!(
        (h_bottom - 100.0).abs() < 0.5,
        "bottom expected ~100 px tall, got {h_bottom}",
    );
}

#[test]
fn nested_splitter_renders_all_leaves() {
    // Outer vertical splitter; bottom pane is itself a horizontal splitter
    // with two sub-panes. Total leaf panes: 1 (top) + 2 (bottom-left,
    // bottom-right) = 3.
    let inner = splitter(SplitterOrientation::Horizontal)
        .child(container().label("bottom-left"), 150.0)
        .child(container().label("bottom-right"), 150.0);

    let mut cx = Cx::new_root();
    let root = cx.add(
        splitter(SplitterOrientation::Vertical)
            .child(container().label("top"), 100.0)
            .child(inner, 100.0),
    );

    let _layout = compute(&mut cx, root, 306.0, 206.0);

    // Outer splitter: top pane (0), handle (1), bottom pane (2).
    let bottom_pane = cx.nth_child(root, 2).expect("bottom pane");
    // Bottom pane wraps the inner splitter as its content; the inner
    // splitter lives one level down.
    let inner_root = cx.nth_child(bottom_pane, 0).expect("inner splitter root");

    // Inner splitter: left (0), handle (1), right (2).
    let left = cx.nth_child(inner_root, 0).expect("bottom-left");
    let right = cx.nth_child(inner_root, 2).expect("bottom-right");

    assert!(cx.get(left).is_some(), "bottom-left leaf must exist");
    assert!(cx.get(right).is_some(), "bottom-right leaf must exist");
}

#[test]
fn pane_builder_carries_initial_min_max_into_layout() {
    // The pane(node).initial(px).min(px).max(px) per-pane builder must
    // produce the same layout result as the positional .child(...)/
    // .min_size(idx, px)/.max_size(idx, px) triplet.
    let mut cx = Cx::new_root();
    let root = cx.add(
        splitter(SplitterOrientation::Horizontal)
            .pane(container().label("a"))
            .initial(150.0)
            .min(50.0)
            .max(300.0)
            .pane(container().label("b"))
            .initial(250.0)
            .min(100.0)
            .max(400.0),
    );

    let layout = compute(&mut cx, root, 406.0, 200.0);

    let pane_a = cx.nth_child(root, 0).expect("pane a");
    let pane_b = cx.nth_child(root, 2).expect("pane b");

    let wa = layout.result(pane_a).expect("pane a layout").rect().width;
    let wb = layout.result(pane_b).expect("pane b layout").rect().width;

    assert!(
        (wa - 150.0).abs() < 0.5,
        "pane a expected ~150 px wide, got {wa}",
    );
    assert!(
        (wb - 250.0).abs() < 0.5,
        "pane b expected ~250 px wide, got {wb}",
    );
}

#[test]
fn grow_pane_takes_leftover_width() {
    sigil::reactive::Runtime::scope("phase-2-4::splitter-grow", || {
        let mut cx = Cx::new_root();
        let root = cx.add(
            splitter(SplitterOrientation::Horizontal)
                .pane(container().label("side"))
                .initial(120.0)
                .pane(container().label("main"))
                .grow(1.0)
                .commit()
                .width(406.0)
                .height(200.0),
        );
        let layout = compute(&mut cx, root, 406.0, 200.0);
        let side = cx.nth_child(root, 0).expect("side pane");
        let main = cx.nth_child(root, 2).expect("main pane");
        let side_w = layout.result(side).expect("side").rect().width;
        let main_w = layout.result(main).expect("main").rect().width;
        assert!(
            (side_w - 120.0).abs() < 0.5,
            "fixed pane stays 120, got {side_w}"
        );
        assert!(
            (main_w - 280.0).abs() < 0.5,
            "grow pane takes leftover (406 - 120 - 6 handle), got {main_w}"
        );
    });
}

#[test]
fn sizes_bind_carries_initial_sizes() {
    sigil::reactive::Runtime::scope("phase-2-4::splitter-sizes-bind", || {
        let bound = signal(Vec::<f32>::new());
        let mut cx = Cx::new_root();
        let _root = cx.add(
            splitter(SplitterOrientation::Horizontal)
                .child(container().label("a"), 100.0)
                .child(container().label("b"), 150.0)
                .child(container().label("c"), 200.0)
                .sizes_bind(bound),
        );

        let snapshot = bound.peek();
        assert_eq!(
            snapshot,
            vec![100.0, 150.0, 200.0],
            "sizes_bind must reflect initial sizes after build",
        );
    });
}
