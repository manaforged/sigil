#![allow(unused_crate_dependencies)]
#![allow(missing_docs)]
#![allow(clippy::float_cmp)]
#![allow(clippy::unwrap_used)]
#![allow(clippy::indexing_slicing)]

//! `position_reactive` binds a node's absolute inset to a signal, repositions
//! on signal change via the layout-signature cache, and keeps the node's
//! identity stable (so an in-flight pointer capture / drag survives).

use ahash as _;
use allocation_counter as _;
use criterion as _;
use rayon as _;
use sigil_hal as _;
use sigil_style as _;
use smallvec as _;
use taffy as _;

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
