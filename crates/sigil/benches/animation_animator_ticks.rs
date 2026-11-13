#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used, clippy::panic)]

use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use sigil::animation::{Animator, Spring, tick};
use sigil::reactive::Owner;

const DT_60HZ: f32 = 1.0 / 60.0;

fn bench_animator_ticks(c: &mut Criterion) {
    let mut group = c.benchmark_group("animator_ticks");
    for &count in &[1_usize, 10, 100, 1000] {
        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, &count| {
            Owner::scope("animator_ticks_bench", || {
                let animators: Vec<Animator> = (0..count)
                    .map(|_| {
                        let a = Animator::spring(0.0_f32, Spring::SMOOTH);
                        a.target(1.0);
                        a
                    })
                    .collect();

                b.iter(|| {
                    for a in &animators {
                        if !a.active().peek() {
                            a.target(black_box(if a.value().peek() > 0.5 { 0.0 } else { 1.0 }));
                        }
                    }
                    tick(black_box(DT_60HZ));
                });

                let _ = black_box(animators);
            });
        });
    }
    group.finish();
}

criterion_group!(benches, bench_animator_ticks);
criterion_main!(benches);
