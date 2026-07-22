//! Criterion benchmark for the `heatmap()` overlay widget.
//!
//! Target: build a `100x100` grid (`10_000` cells) in <= 500us.

#![allow(missing_docs, clippy::panic)]

use arboard as _;
use image as _;
use pollster as _;
use sigil_actions as _;
use sigil_animation as _;
use sigil_animation_runtime as _;
use sigil_app as _;
use sigil_bidi as _;
use sigil_events as _;
use sigil_hal as _;
use sigil_renderer as _;
use sigil_tasks as _;
use std::hint::black_box;
use wgpu as _;

use criterion::{Criterion, criterion_group, criterion_main};
use sigil::widgets::overlay::heatmap;
use sigil_kernel::Cx;
use sigil_reactive::Runtime;
use sigil_style::Rgba8;

// Silence unused-crate-dependencies for dev-only crates the sigil
// facade pulls in.
use sigil_layout as _;

const fn cell_color(x: usize, y: usize) -> Rgba8 {
    Rgba8 {
        #[allow(clippy::cast_possible_truncation)]
        r: (x & 0xff) as u8,
        #[allow(clippy::cast_possible_truncation)]
        g: (y & 0xff) as u8,
        b: 128,
        a: 255,
    }
}

fn bench_heatmap_100x100(c: &mut Criterion) {
    c.bench_function("heatmap_100x100", |b| {
        b.iter(|| {
            Runtime::scope("heatmap-bench", || {
                let mut cx = Cx::new_root();
                let widget = heatmap(black_box(100), black_box(100), cell_color);
                let _ = black_box(cx.add(widget));
            });
        });
    });
}

criterion_group!(benches, bench_heatmap_100x100);
criterion_main!(benches);
