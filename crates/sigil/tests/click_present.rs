//! Headless increment-to-present: hello-counter Increment.
//!
//! Times `signal.set` + layout + scene emit — the path `Host` and a
//! game loop take after a handler. Not a pointer event and not a
//! presented wgpu frame. Native bar is `FRAME_BUDGET_NS` (16.67 ms).

#![allow(missing_docs, unused_crate_dependencies, clippy::unwrap_used)]

use std::time::Instant;

use sigil::Host;
use sigil::prelude::*;
use sigil::profile::FRAME_BUDGET_NS;

const SAMPLES: usize = 64;

#[test]
fn increment_layout_emit_under_frame_budget() {
    Runtime::scope("click-present", || {
        let count = signal(0_i32);
        let label = derived(move || format!("Count: {}", count.get()));
        let mut host = Host::new();
        host.build(|cx| {
            cx.add(
                container()
                    .layout(
                        LayoutStyle::flex_column()
                            .fixed_size(480.0, 240.0)
                            .gap(16.0),
                    )
                    .child(text(label))
                    .child(text("Increment")),
            )
        });
        let viewport = (480.0, 240.0);
        drop(host.scene(viewport).unwrap());

        let mut samples = Vec::with_capacity(SAMPLES);
        for n in 1..=SAMPLES {
            let started = Instant::now();
            count.set(i32::try_from(n).unwrap());
            drop(host.scene(viewport).unwrap());
            samples.push(started.elapsed().as_nanos());
        }
        samples.sort_unstable();
        let median = samples.get(SAMPLES / 2).copied().expect("64 samples");
        let mean = samples.iter().sum::<u128>() / samples.len() as u128;
        let var = samples
            .iter()
            .map(|s| {
                let d = i128::try_from(*s).unwrap() - i128::try_from(mean).unwrap();
                u128::try_from(d.saturating_mul(d)).unwrap()
            })
            .sum::<u128>()
            / samples.len() as u128;
        #[allow(clippy::cast_precision_loss)]
        let cv = (var as f64).sqrt() / (mean as f64).max(1.0);
        eprintln!(
            "click-to-present Increment n={SAMPLES} median={}us mean={}us cv={cv:.3}",
            median / 1000,
            mean / 1000
        );
        assert!(
            median < u128::from(FRAME_BUDGET_NS),
            "median {median} ns exceeds FRAME_BUDGET_NS ({FRAME_BUDGET_NS}); cv={cv:.3}"
        );
        assert!(cv < 1.0, "cv {cv:.3} is too noisy to quote");
    });
}
