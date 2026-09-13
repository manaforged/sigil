#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(
    missing_docs,
    clippy::missing_docs_in_private_items,
    clippy::unwrap_used,
    clippy::cast_precision_loss
)]

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use sigil::hal::{LogicalSize, Point, Rect, Size};
use sigil::kernel::{Cx, NodeId, TextContent};
use sigil::layout::{LayoutEngine, TextMeasure};
use sigil::prelude::*;
use sigil::profile;
use sigil::reactive::Runtime;
use sigil::renderer::emit_scene_clipped;

const ROWS: u32 = 500;

struct StubMeasure;

impl TextMeasure for StubMeasure {
    fn measure(&self, _: &TextContent, _: Size) -> LogicalSize {
        LogicalSize::new(80.0, 16.0)
    }
}

fn build(cx: &mut Cx) -> NodeId {
    let mut col = container().layout(LayoutStyle::flex_column().fixed_size(1280.0, 800.0));
    for index in 0..ROWS {
        col = col.child(
            container()
                .layout(LayoutStyle::flex_row().fixed_size(1280.0, 24.0).shrink(0.0))
                .child(text(format!("row {index}"))),
        );
    }
    cx.add(col)
}

fn layout_then_emit(c: &mut Criterion) {
    profile::enable();
    c.bench_function("frame_phases/layout_emit_500", |b| {
        b.iter(|| {
            Runtime::scope("frame_phases", || {
                let mut cx = Cx::new_root();
                let root = build(&mut cx);
                let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
                {
                    let _layout = profile::span("layout");
                    layout
                        .compute(&mut cx, root, Size::fixed(1280.0, 800.0))
                        .expect("bench harness invariant");
                }
                {
                    let _emit = profile::span("emit");
                    black_box(emit_scene_clipped(
                        &cx,
                        root,
                        Some(Rect::new(Point::new(0.0, 0.0), 1280.0, 800.0)),
                    ));
                }
                profile::tick_frame();
            });
        });
    });
    profile::disable();
}

criterion_group!(benches, layout_then_emit);
criterion_main!(benches);
