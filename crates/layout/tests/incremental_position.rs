//! Regression — incremental remeasure must preserve a fixed-size subtree's
//! in-tree position, not just its size.
//!
//! When a deep node goes dirty, the engine takes the incremental path: it walks
//! up to the nearest fixed-size ancestor, treats it as a containment root, and
//! re-solves that subtree in isolation via `measure_with_constraints` — which
//! anchors the root at (0,0). For a root that is *not* the tree root, that is
//! position-destroying without the origin-restore fold: the fixed-size box
//! escapes to the window's top-left.
//!
//! This is exactly what made the chat composer's text field render typed text
//! in the corner of the window. The field's text region is a fixed-size
//! containment root, and every keystroke rebuilds the caret/selection overlay
//! (a `children_reactive` subtree) — a *structural* mutation whose sole
//! containment root is the region itself, with no ancestor root to fall back
//! on. The earlier multi-root fix (drop roots nested under another) can't help
//! a lone root.
//!
//! The tree mirrors that shape: a tall flex column pushes a fixed-size region
//! down to a known y, the region holds a `children_reactive` body, and a fan of
//! inert fillers keeps the dirty set small enough to force the incremental
//! branch. Toggling the body must leave the region exactly where its unchanged
//! parent placed it.

#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(missing_docs)]
#![allow(clippy::unwrap_used, clippy::float_cmp)]

mod stub;

use sigil_hal::Size;
use sigil_kernel::prelude::*;
use sigil_layout::prelude::*;
use sigil_reactive::{Runtime, signal};
use stub::stub_engine;

const AVAILABLE: Size = Size::fixed(200.0, 1000.0);
/// The spacer height — and therefore the region's expected top in the column.
const SPACER_H: f32 = 120.0;

#[test]
// Upstream test (6938732) arrived at 70 lines; splitting the build/toggle/
// assert phases would obscure the single scenario it walks.
#[allow(clippy::too_many_lines)]
fn incremental_structural_remeasure_keeps_fixed_region_at_its_position() {
    Runtime::scope("incremental_remeasure_position", || {
        let toggle = signal(false);
        let mut cx = Cx::new_root();
        // flex_column root: spacer → region → 20 inert fillers. The fillers
        // inflate the live-node count so the one-subtree mutation stays below
        // the broadcast threshold (dirty < live/2) and takes the incremental
        // path under test.
        //
        // CRUCIAL: the `children_reactive` lives on a NON-fixed wrapper *inside*
        // the fixed-size region, not on the region itself. The structural
        // mutation is therefore deep — its sole containment root walks up to the
        // region (the first fixed-size ancestor) with NO ancestor root to fall
        // back on, exactly like the text field's caret overlay buried under the
        // absolute track. (Putting `children_reactive` on the region directly
        // would dirty the region itself, promoting its containment root to the
        // tree root — which a root-anchored remeasure positions correctly,
        // masking the bug.)
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
            // A `display: none` child inside the region's subtree: the solver
            // skips it, so it carries a stale box + old frame tag. The subtree
            // translate MUST leave it unreached (frame-checked read), not
            // resurrect it at a shifted position.
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

        // Rebuild the region's body — a structural mutation whose sole
        // containment root is the fixed-size region, forcing the isolated
        // remeasure that the fix must reposition.
        toggle.set(true);
        engine
            .compute(&mut cx, root, AVAILABLE)
            .expect("incremental structural compute succeeds");

        assert_ne!(
            engine.generation(),
            generation,
            "structural mutation must invalidate the layout cache (incremental path ran)",
        );
        // Paint path: the `LayoutResult` attached to the kernel node.
        assert_eq!(
            region_top(&cx, region),
            SPACER_H,
            "region (paint) must stay put after incremental remeasure, not escape to y=0",
        );
        // Hit-test path: `engine.result()` reads the live drive box, which
        // hit_test.rs + pointer_dispatch.rs consume. It MUST match paint, or
        // the region paints in place but clicks resolve at the top-left
        //.
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
        // The display:none child must stay unreached — the translate skips it
        // via the frame-checked read, so no live (shifted) box resurrects it.
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
