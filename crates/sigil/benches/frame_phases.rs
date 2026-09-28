#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used, clippy::cast_precision_loss)]

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use sigil::hal::{Point, Rect, Size};
use sigil::kernel::{Cx, NodeId};
use sigil::layout::LayoutEngine;
use sigil::prelude::*;
use sigil::profile;
use sigil::reactive::Runtime;
use sigil::scene::emit_scene_clipped;
use sigil_kittest::FixedMeasure;

const ROWS: u32 = 500;

const TEXT_MEASURE: FixedMeasure = FixedMeasure {
    width: 80.0,
    height: 16.0,
};

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
                let mut layout = LayoutEngine::with_text_measure(Box::new(TEXT_MEASURE));
                {
                    let _layout = profile::span(sigil::profile::Phase::Layout);
                    layout
                        .compute(&mut cx, root, Size::fixed(1280.0, 800.0))
                        .expect("bench harness invariant");
                }
                {
                    let _emit = profile::span(sigil::profile::Phase::Emit);
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
