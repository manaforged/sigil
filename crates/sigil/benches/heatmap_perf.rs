#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(missing_docs, clippy::panic)]

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use sigil::kernel::Cx;
use sigil::reactive::Runtime;
use sigil::style::Rgba8;
use sigil::widgets::overlay::heatmap;

const fn cell_color(x: usize, y: usize) -> Rgba8 {
    Rgba8 {
        #[allow(
            clippy::cast_possible_truncation,
            reason = "values are bounded to the target range before the cast"
        )]
        r: (x & 0xff) as u8,
        #[allow(
            clippy::cast_possible_truncation,
            reason = "values are bounded to the target range before the cast"
        )]
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
