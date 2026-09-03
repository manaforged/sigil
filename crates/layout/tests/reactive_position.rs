#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(missing_docs)]
#![allow(clippy::float_cmp)]
#![allow(clippy::unwrap_used)]
#![allow(clippy::indexing_slicing)]

//! `position_reactive` binds a node's absolute inset to a signal, repositions
//! on signal change via the layout-signature cache, and keeps the node's
//! identity stable (so an in-flight pointer capture / drag survives).

mod stub;
use stub::stub_engine;

use sigil_kernel::prelude::*;
use sigil_layout::prelude::*;
use sigil_reactive::{Runtime, signal};

fn rect(cx: &Cx, id: NodeId) -> sigil_hal::Rect {
    cx.get(id)
        .and_then(|node| node.get_extension::<LayoutResult>())
        .expect("rect")
        .rect()
}

#[test]
fn position_reactive_tracks_signal_with_stable_identity() {
    Runtime::scope("layout_reactive_position", || {
        let pos = signal((50.0_f32, 30.0_f32));
        let mut cx = Cx::new_root();
        let root = cx.add(
            container()
                .layout(LayoutStyle::flex_column().fixed_size(800.0, 600.0))
                .child(
                    container()
                        .layout(LayoutStyle::flex_column().fixed_size(100.0, 40.0))
                        .position_reactive(pos),
                ),
        );
        let node = *cx.get(root).unwrap().children().first().unwrap();

        let mut engine = stub_engine();
        engine
            .compute(&mut cx, root, Size::fixed(800.0, 600.0))
            .expect("initial compute");
        let r1 = rect(&cx, node);
        assert_eq!(
            (r1.origin.x, r1.origin.y),
            (50.0, 30.0),
            "node sits at the signal's initial inset; got {:?}",
            (r1.origin.x, r1.origin.y),
        );

        // Move the signal — the node must follow on the next compute.
        pos.set((120.0, 200.0));
        engine
            .compute(&mut cx, root, Size::fixed(800.0, 600.0))
            .expect("reposition compute");
        let r2 = rect(&cx, node);
        assert_eq!(
            (r2.origin.x, r2.origin.y),
            (120.0, 200.0),
            "node follows the signal to its new inset; got {:?}",
            (r2.origin.x, r2.origin.y),
        );

        // Identity is stable across the reposition — no rebuild, so a live
        // pointer capture on this node would survive a drag.
        let node_again = *cx.get(root).unwrap().children().first().unwrap();
        assert_eq!(
            node, node_again,
            "node identity must be stable across reposition (capture-safe)",
        );
    });
}

#[test]
fn bounds_reactive_publishes_the_absolute_rect() {
    Runtime::scope("layout_bounds_reactive", || {
        let bounds = signal((0.0_f32, 0.0_f32, 0.0_f32, 0.0_f32));
        let mut cx = Cx::new_root();
        let root = cx.add(
            container()
                .layout(
                    LayoutStyle::flex_column()
                        .fixed_size(800.0, 600.0)
                        .padding(10.0),
                )
                .child(
                    container()
                        .layout(LayoutStyle::flex_column().fixed_size(100.0, 40.0))
                        .bounds_reactive(bounds),
                ),
        );

        let mut engine = stub_engine();
        engine
            .compute(&mut cx, root, Size::fixed(800.0, 600.0))
            .expect("initial compute");
        assert_eq!(
            bounds.peek(),
            (10.0, 10.0, 100.0, 40.0),
            "the bound signal carries the absolute border box after compute",
        );
    });
}

#[test]
fn reposition_translates_the_subtree_without_a_resolve() {
    Runtime::scope("layout_reactive_translate", || {
        let pos = signal((0.0_f32, 0.0_f32));
        let mut cx = Cx::new_root();
        let root = cx.add(
            container()
                .layout(LayoutStyle::flex_column().fixed_size(800.0, 600.0))
                .child(
                    container()
                        .layout(LayoutStyle::flex_column().fixed_size(100.0, 40.0))
                        .position_reactive(pos)
                        .child(container().layout(LayoutStyle::block().fixed_size(10.0, 10.0))),
                ),
        );
        let strip = *cx.get(root).unwrap().children().first().unwrap();
        let leaf = *cx.get(strip).unwrap().children().first().unwrap();
        let mut engine = stub_engine();
        engine
            .compute(&mut cx, root, Size::fixed(800.0, 600.0))
            .expect("initial compute");
        sigil_profile::enable();
        sigil_profile::reset();
        sigil_profile::begin_frame();
        pos.set((0.0, -35.0));
        engine
            .compute(&mut cx, root, Size::fixed(800.0, 600.0))
            .expect("translate compute");
        sigil_profile::tick_frame();
        let counts = sigil_profile::frames()
            .last()
            .map(|f| f.counts.clone())
            .unwrap_or_default();
        assert!(
            counts.iter().any(|(name, _)| *name == "layout.translate"),
            "a position-only change must take the translate path; counts {counts:?}"
        );
        assert_eq!(rect(&cx, strip).origin.y, -35.0);
        assert_eq!(rect(&cx, leaf).origin.y, -35.0);
    });
}

#[test]
fn reposition_with_a_dirty_descendant_still_translates() {
    Runtime::scope("layout_reactive_translate_dirty_child", || {
        let pos = signal((0.0_f32, 0.0_f32));
        let label = signal(String::from("a"));
        let mut cx = Cx::new_root();
        let root = cx.add(
            container()
                .layout(LayoutStyle::flex_column().fixed_size(800.0, 600.0))
                .child(
                    container()
                        .layout(LayoutStyle::flex_column().fixed_size(100.0, 40.0))
                        .position_reactive(pos)
                        .child(
                            container()
                                .layout(LayoutStyle::block().fixed_size(50.0, 10.0))
                                .child(text_live(move || label.get())),
                        ),
                ),
        );
        let strip = *cx.get(root).unwrap().children().first().unwrap();
        let mut engine = stub_engine();
        engine
            .compute(&mut cx, root, Size::fixed(800.0, 600.0))
            .expect("initial compute");
        sigil_profile::enable();
        sigil_profile::reset();
        sigil_profile::begin_frame();
        label.set(String::from("bb"));
        pos.set((0.0, -35.0));
        engine
            .compute(&mut cx, root, Size::fixed(800.0, 600.0))
            .expect("second compute");
        sigil_profile::tick_frame();
        let counts = sigil_profile::frames()
            .last()
            .map(|f| f.counts.clone())
            .unwrap_or_default();
        assert!(
            counts.iter().any(|(name, _)| *name == "layout.translate"),
            "the strip must translate even when a cell inside it is dirty; counts {counts:?}"
        );
        assert_eq!(rect(&cx, strip).origin.y, -35.0);
    });
}

#[test]
fn incremental_root_with_margin_keeps_its_place() {
    Runtime::scope("layout_margin_root", || {
        let label = signal(String::from("a"));
        let mut cx = Cx::new_root();
        let root = cx.add(
            container()
                .layout(LayoutStyle::flex_column().fixed_size(400.0, 300.0))
                .child(container().layout(LayoutStyle::block().fixed_size(100.0, 20.0)))
                .child(
                    container()
                        .layout(LayoutStyle::block().fixed_size(120.0, 40.0).margin(10.0))
                        .child(text_live(move || label.get())),
                ),
        );
        let boxed = *cx.get(root).unwrap().children().get(1).unwrap();
        let mut engine = stub_engine();
        engine
            .compute(&mut cx, root, Size::fixed(400.0, 300.0))
            .expect("initial compute");
        let before = rect(&cx, boxed);
        label.set(String::from("bb"));
        engine
            .compute(&mut cx, root, Size::fixed(400.0, 300.0))
            .expect("incremental compute");
        let after = rect(&cx, boxed);
        assert_eq!(
            (after.origin.x, after.origin.y),
            (before.origin.x, before.origin.y),
            "a margin root must not drift when re-solved on its own"
        );
    });
}
