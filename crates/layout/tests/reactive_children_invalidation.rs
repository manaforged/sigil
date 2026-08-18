//! Regression tests for `children_reactive` × layout engine integration.
//!
//! These tests verify that:
//!
//! 1. Reactive child mutations inside a fixed-size container only relayout
//!    the containing subtree — the outer (intrinsic-sized) ancestor is not
//!    remeasured.
//! 2. An identical rebuild (same child content, same count) hits the layout
//!    cache and does not bump `engine.generation()`.
//! 3. `LayoutEngine::compute` itself drains the deferred rebuild queue via
//!    the `cx.flush_reactive_children()` call wired in B1.b.
//!
//! The canonical model test is
//! `compute_after_add_child_invalidates_cache` in `incremental.rs`.
#![allow(unused_crate_dependencies)]
#![allow(missing_docs)]
#![allow(clippy::unwrap_used, clippy::float_cmp)]

use ahash as _;
use allocation_counter as _;
use criterion as _;
use rayon as _;
use sigil_hal as _;
use sigil_style as _;
use smallvec as _;

mod stub;
use stub::stub_engine;

use sigil_hal::Size;
use sigil_kernel::prelude::*;
use sigil_layout::prelude::*;
use sigil_reactive::{Runtime, signal};

/// Available space fed to the layout engine in each test.
const AVAILABLE: Size = Size::fixed(800.0, 1000.0);

fn layout_rect(cx: &Cx, id: NodeId) -> sigil_hal::Rect {
    cx.get(id)
        .and_then(|node| node.get_extension::<LayoutResult>())
        .expect("layout result must exist")
        .rect()
}

/// A `children_reactive` parent inside a fixed-size container must not
/// trigger a root-level re-layout when children are added.
///
/// Tree shape:
///
/// ```text
/// outer  (flex_column, no fixed size — intrinsic-sized)
///   middle (fixed_size 400x600 — containment root)
///     inner  (flex_column, no fixed size)
///       [reactive children]
/// ```
///
/// When inner's children grow, `find_containment_root` stops at `middle`
/// (the first Px-sized ancestor). The outer is NOT remeasured, so its
/// rect is identical before and after the mutation.
#[test]
fn reactive_children_in_sized_container_only_relayouts_subtree() {
    Runtime::scope("layout_reactive_children_sized_container_subtree", || {
        let items = signal(vec!["a", "b"]);
        let mut cx = Cx::new_root();

        let inner_items = items;
        let outer = cx.add(
            container().layout(LayoutStyle::flex_column()).child(
                container()
                    .layout(LayoutStyle::flex_column().fixed_size(400.0, 600.0))
                    .child(
                        container()
                            .layout(LayoutStyle::flex_column())
                            .children_reactive(move || {
                                inner_items.get().into_iter().map(|_s| {
                                    container().layout(LayoutStyle::block().fixed_size(100.0, 20.0))
                                })
                            }),
                    ),
            ),
        );

        let mut engine = stub_engine();
        engine
            .compute(&mut cx, outer, AVAILABLE)
            .expect("initial compute succeeds");

        let gen_after_initial = engine.generation();
        let outer_rect_initial = layout_rect(&cx, outer);

        // Add 2 more items — structural change inside the fixed-size subtree.
        items.set(vec!["a", "b", "c", "d"]);

        engine
            .compute(&mut cx, outer, AVAILABLE)
            .expect("reactive compute succeeds");

        // Generation must advance — a structural change happened.
        assert_ne!(
            engine.generation(),
            gen_after_initial,
            "engine generation must advance after children_reactive mutation"
        );

        // Outer's rect must be unchanged because the remeasure is
        // contained inside `middle` (the fixed-size ancestor).
        let outer_rect_after = layout_rect(&cx, outer);
        assert_eq!(
            outer_rect_after.width.to_bits(),
            outer_rect_initial.width.to_bits(),
            "outer width must not change (containment root was middle)"
        );
        assert_eq!(
            outer_rect_after.height.to_bits(),
            outer_rect_initial.height.to_bits(),
            "outer height must not change (containment root was middle)"
        );
    });
}

/// `Signal::set` skips an equal write, so an identical item list does
/// not rebuild children or invalidate layout.
#[test]
fn reactive_children_identical_set_is_a_cache_hit() {
    Runtime::scope("layout_reactive_children_identical_rebuild_cache", || {
        let items = signal(vec!["a", "b", "c"]);
        let mut cx = Cx::new_root();

        let root = cx.add(
            container()
                .layout(LayoutStyle::flex_column().fixed_size(300.0, 400.0))
                .children_reactive(move || {
                    items
                        .get()
                        .into_iter()
                        .map(|_s| container().layout(LayoutStyle::block().fixed_size(100.0, 20.0)))
                }),
        );

        let mut engine = stub_engine();
        engine
            .compute(&mut cx, root, AVAILABLE)
            .expect("initial compute succeeds");

        let gen_after_initial = engine.generation();

        items.set(vec!["a", "b", "c"]);

        engine
            .compute(&mut cx, root, AVAILABLE)
            .expect("identical set compute succeeds");

        assert_eq!(
            engine.generation(),
            gen_after_initial,
            "equal Signal::set does not rebuild children or re-layout"
        );
    });
}

/// `LayoutEngine::compute` must drain the reactive-children queue via
/// `cx.flush_reactive_children()` WITHOUT the caller doing it explicitly.
///
/// Build a tree with `children_reactive`, mutate the signal (queuing a
/// rebuild), then call `engine.compute` without a manual flush — the
/// children must materialise, proving the B1.b hookup works end-to-end.
#[test]
fn reactive_children_dispatched_through_compute() {
    Runtime::scope("layout_reactive_children_via_compute", || {
        let items = signal(Vec::<&str>::new());
        let mut cx = Cx::new_root();

        let root = cx.add(
            container()
                .layout(LayoutStyle::flex_column().fixed_size(300.0, 400.0))
                .children_reactive(move || {
                    items
                        .get()
                        .into_iter()
                        .map(|_s| container().layout(LayoutStyle::block().fixed_size(100.0, 20.0)))
                }),
        );

        let mut engine = stub_engine();

        // Initial compute — empty list, no children.
        engine
            .compute(&mut cx, root, AVAILABLE)
            .expect("initial compute succeeds");

        assert_eq!(
            cx.get(root).expect("root alive").children().len(),
            0,
            "no children in the initial empty build"
        );

        // Queue a rebuild — do NOT call flush_reactive_children explicitly.
        items.set(vec!["x", "y", "z"]);

        // compute() must drain the queue internally.
        engine
            .compute(&mut cx, root, AVAILABLE)
            .expect("compute with queued rebuild succeeds");

        assert_eq!(
            cx.get(root).expect("root alive").children().len(),
            3,
            "compute must drain the reactive-children queue and materialise 3 children"
        );
    });
}
