//! The dirty-frame cache short-circuit must never strand a stale layout.
//!
//! `resolve_cache_key` skips the O(nodes) input-signature walk on a dirty
//! frame (it is already a guaranteed MISS) and carries the previous
//! signature forward. This pins that a mutation is still reflected on the
//! very next compute, and that a subsequent unchanged frame becomes a cache
//! HIT (the generation counter stops advancing) rather than re-laying-out or,
//! worse, serving a stale box.

#![allow(missing_docs, unused_crate_dependencies, clippy::unwrap_used)]

use sigil_hal::Size;
use sigil_kernel::{Cx, container};
use sigil_layout::prelude::{LayoutBuilderExt, LayoutStyle};
use sigil_layout::{LayoutEngine, TextMeasure};

struct NoText;
impl TextMeasure for NoText {
    fn measure(&self, _: &sigil_kernel::TextContent, _: Size) -> sigil_hal::LogicalSize {
        sigil_hal::LogicalSize::new(0.0, 0.0)
    }
}

#[test]
fn dirty_frame_reflects_mutation_then_clean_frame_hits() {
    let mut cx = Cx::new_root();
    let leaf = container().layout(LayoutStyle::block().fixed_size(40.0, 10.0));
    let root = cx.add(
        container()
            .layout(LayoutStyle::flex_column().fixed_size(200.0, 200.0))
            .child(leaf),
    );
    let leaf_id = *cx.get(root).unwrap().children().first().unwrap();

    let mut engine = LayoutEngine::with_text_measure(Box::new(NoText));
    let avail = Size::fixed(200.0, 200.0);
    engine.compute(&mut cx, root, avail).unwrap();
    cx.clear_dirty();
    assert!((engine.result(leaf_id).unwrap().rect().width - 40.0).abs() < 0.5);

    // Mutate the leaf (dirty frame): the new width MUST be reflected, not
    // stale from a wrongly-served cache hit.
    cx.get_mut(leaf_id)
        .unwrap()
        .attach(LayoutStyle::block().fixed_size(80.0, 10.0));
    engine.compute(&mut cx, root, avail).unwrap();
    cx.clear_dirty();
    assert!(
        (engine.result(leaf_id).unwrap().rect().width - 80.0).abs() < 0.5,
        "the dirty-frame mutation must be reflected; got {}",
        engine.result(leaf_id).unwrap().rect().width,
    );

    // Two clean frames with nothing changed: the second must be a cache HIT
    // (generation frozen), proving the carried-forward signature converges.
    engine.compute(&mut cx, root, avail).unwrap();
    let g1 = engine.generation();
    engine.compute(&mut cx, root, avail).unwrap();
    let g2 = engine.generation();
    assert_eq!(
        g1, g2,
        "an unchanged clean frame must be a cache hit, not a recompute"
    );
    assert!((engine.result(leaf_id).unwrap().rect().width - 80.0).abs() < 0.5);
}
