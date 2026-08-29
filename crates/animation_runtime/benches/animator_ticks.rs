#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
//! Animator-tick micro-benchmark — baseline.
//!
//! Establishes the per-frame cost of [`tick(dt)`](sigil_animation_runtime::tick)
//! at varying active-animator counts (1, 10, 100, 1000). The
//! binding-install hook routes every widget's state transitions through
//! `Animator::tick`, so this curve is the regression baseline.
//!
//! Run with `cargo bench -p sigil-animation-runtime --bench animator_ticks`.
//! Compile-only smoke: `cargo bench -p sigil-animation-runtime
//! --bench animator_ticks -- --test`.

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
                // Build `count` animators with the Smooth preset spring,
                // each retargeted to a non-settled value so tick does real
                // work every frame. Hold them in a Vec to keep them alive
                // across iterations (Drop deregisters from the active set).
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
                    // Re-arm any animator that has settled so subsequent
                    // ticks keep doing real work. Cheap on still-running
                    // animators; the spring just notes the new target.
                    for a in &mut animators {
                        if a.settled() {
                            a.target(black_box(1.0));
                        }
                    }
                    tick(black_box(DT_60HZ));
                });

                // Explicit drop so the active set is empty before the
                // next parameter sweep. (Drop runs at scope end anyway,
                // but being explicit keeps the bench fence-clean.)
                drop(animators);
            });
        });
    }
    group.finish();
}

criterion_group!(benches, bench_animator_ticks);
criterion_main!(benches);
