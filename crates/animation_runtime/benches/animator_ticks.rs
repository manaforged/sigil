#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(
    missing_docs,
    clippy::missing_docs_in_private_items,
    clippy::unwrap_used,
    clippy::panic
)]

use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use sigil_animation::SpringPreset;
use sigil_animation_runtime::{AnimationConfig, Animator, tick};
use sigil_reactive::{Runtime, signal};

const DT_60HZ: f32 = 1.0 / 60.0;

fn bench_animator_ticks(c: &mut Criterion) {
    let mut group = c.benchmark_group("animator_ticks");
    for &count in &[1_usize, 10, 100, 1000] {
        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, &count| {
            Runtime::scope("animator_ticks_bench", || {
                let mut animators: Vec<Animator> = (0..count)
                    .map(|_| {
                        let s = signal(0.0_f32);
                        let mut a = Animator::new(
                            s,
                            AnimationConfig::Spring {
                                preset: SpringPreset::Smooth,
                            },
                        );
                        a.target(1.0);
                        a
                    })
                    .collect();

                b.iter(|| {
                    for a in &mut animators {
                        if a.settled() {
                            a.target(black_box(1.0));
                        }
                    }
                    tick(black_box(DT_60HZ));
                });

                drop(animators);
            });
        });
    }
    group.finish();
}

criterion_group!(benches, bench_animator_ticks);
criterion_main!(benches);
