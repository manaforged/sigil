#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used, clippy::float_cmp)]

#[path = "layout_stub/mod.rs"]
mod stub;

use sigil::geometry::Size;
use sigil::kernel::*;
use sigil::layout::*;
use sigil::reactive::{Runtime, signal};
use stub::stub_engine;

const AVAILABLE: Size = Size::fixed(200.0, 1000.0);
const SPACER_H: f32 = 120.0;

#[test]
#[allow(clippy::too_many_lines)]
fn incremental_structural_remeasure_keeps_fixed_region_at_its_position() {
    Runtime::scope("incremental_remeasure_position", || {
        let toggle = signal(false);
        let mut cx = Cx::new_root();
        let region_body = container()
            .layout(LayoutStyle::block().fixed_size(180.0, 16.0))
            .child(
                container()
                    .layout(LayoutStyle::block())
                    .children_reactive(move || {
                        let n = if toggle.get() { 2 } else { 1 };
                        (0..n)
                            .map(|_| container().layout(LayoutStyle::block().fixed_size(8.0, 8.0)))
                            .collect::<Vec<_>>()
                    }),
            )
            .child(container().layout(LayoutStyle::block().display(Display::None)));
        let tree = container()
            .layout(LayoutStyle::flex_column().fixed_size(200.0, 1000.0))
            .child(container().layout(LayoutStyle::block().fixed_size(200.0, SPACER_H)))
            .child(region_body);
        let root = cx.add(tree);
        let region = cx.nth_child(root, 1).expect("region is second child");
        let hidden = cx
            .nth_child(region, 1)
            .expect("region has a hidden second child");
        for _ in 0..20 {
            let _ = cx.add_child(
                root,
                container().layout(LayoutStyle::block().fixed_size(10.0, 4.0)),
            );
        }
        let mut engine = stub_engine();

        engine
            .compute(&mut cx, root, AVAILABLE)
            .expect("initial compute succeeds");
        assert_eq!(
            region_top(&cx, region),
            SPACER_H,
            "region starts below the spacer",
        );
        let generation = engine.generation();

        toggle.set(true);
        engine
            .compute(&mut cx, root, AVAILABLE)
            .expect("incremental structural compute succeeds");

        assert_ne!(
            engine.generation(),
            generation,
            "structural mutation must invalidate the layout cache (incremental path ran)",
        );
        assert_eq!(
            region_top(&cx, region),
            SPACER_H,
            "region (paint) must stay put after incremental remeasure, not escape to y=0",
        );
        assert_eq!(
            engine
                .result(region)
                .expect("region has a drive result")
                .rect()
                .origin
                .y,
            SPACER_H,
            "region (hit-test / drive) escaped to y=0 — paint/hit-test split",
        );
        assert!(
            engine.result(hidden).is_none(),
            "display:none node was resurrected by the subtree translate",
        );
    });
}

fn region_top(cx: &Cx, id: NodeId) -> f32 {
    cx.get(id)
        .and_then(|node| node.get_extension::<LayoutResult>())
        .expect("region has a layout result")
        .rect()
        .origin
        .y
}
