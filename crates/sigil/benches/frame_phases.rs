#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used, clippy::cast_precision_loss)]

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use sigil::geometry::{Constraint, Point, Rect};
use sigil::kernel::Cx;
use sigil::layout::LayoutEngine;
use sigil::prelude::*;
use sigil::profile;
use sigil::reactive::Owner;
use sigil::scene::emit_scene;
use sigil_kittest::FakeMeasure;

const ROWS: u32 = 500;

const TEXT_MEASURE: FakeMeasure = FakeMeasure::fixed(80.0, 16.0);

fn build() -> impl IntoNode {
    let mut col = container().layout(LayoutStyle::flex_column().fixed_size(1280.0, 800.0));
    for index in 0..ROWS {
        col = col.child(
            container()
                .layout(LayoutStyle::flex_row().fixed_size(1280.0, 24.0).shrink(0.0))
                .child(text(format!("row {index}"))),
        );
    }
    col
}

fn layout_then_emit(c: &mut Criterion) {
    profile::enable();
    c.bench_function("frame_phases/layout_emit_500", |b| {
        b.iter(|| {
            Owner::scope("frame_phases", || {
                let mut cx = Cx::new_root();
                let root = cx.add(build);
                let mut layout = LayoutEngine::new(Box::new(TEXT_MEASURE));
                {
                    let _layout = profile::span(sigil::profile::Phase::Layout);
                    layout
                        .compute(&mut cx, root, Constraint::fixed(1280.0, 800.0))
                        .expect("bench harness invariant");
                }
                {
                    let _emit = profile::span(sigil::profile::Phase::Emit);
                    black_box(emit_scene(
                        &cx,
                        root,
                        Some(Rect::new(Point::new(0.0, 0.0), 1280.0, 800.0)),
                    ));
                }
                profile::end_frame();
            });
        });
    });
    profile::disable();
}

criterion_group!(benches, layout_then_emit);
criterion_main!(benches);
