#![allow(unused_crate_dependencies, clippy::unwrap_used)]
use criterion::{Criterion, criterion_group, criterion_main};
use sigil::hal::{LogicalSize, Size};
use sigil::kernel::{Cx, NodeId, TextContent, container};
use sigil::layout::prelude::{LayoutBuilderExt, LayoutStyle};
use sigil::layout::{LayoutEngine, TextMeasure};
use std::hint::black_box;
struct StubMeasure;
impl TextMeasure for StubMeasure {
    fn measure(&self, _: &TextContent, _: sigil::hal::Size) -> LogicalSize {
        LogicalSize::new(0.0, 0.0)
    }
}
const CHILDREN: usize = 1_000;
const LEAVES: usize = 100;
fn build(cx: &mut Cx) -> NodeId {
    let mut root = container().layout(LayoutStyle::flex_row().fixed_size(12800.0, 100.0));
    for _ in 0..CHILDREN {
        let mut col = container().layout(LayoutStyle::flex_column().fixed_size(12.0, 100.0));
        for _ in 0..LEAVES {
            col = col.child(container().layout(LayoutStyle::block().fixed_size(12.0, 1.0)));
        }
        root = root.child(col);
    }
    cx.add(root)
}
fn bench_resize(c: &mut Criterion) {
    c.bench_function("resize/owned", |b| {
        let mut cx = Cx::new_root();
        let root = build(&mut cx);
        let mut engine = LayoutEngine::with_text_measure(Box::new(StubMeasure));
        engine
            .compute(&mut cx, root, Size::fixed(12800.0, 100.0))
            .expect("bench harness invariant");
        cx.clear_dirty();
        let mut w = 12800.0_f32;
        b.iter(|| {
            w = if w >= 12810.0 { 12800.0 } else { w + 1.0 };
            engine
                .compute(&mut cx, root, Size::fixed(w, 100.0))
                .expect("bench harness invariant");
            black_box(engine.result(root));
        });
    });
}
criterion_group!(benches, bench_resize);
criterion_main!(benches);
