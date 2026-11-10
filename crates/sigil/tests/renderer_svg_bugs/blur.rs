use std::time::{Duration, Instant};

use sigil::kernel::container;
use sigil::layout::LayoutStyle;
use sigil::prelude::*;
use sigil_kittest::Harness;

const SIDE: u32 = 1024;
const CAPPED: f32 = 300.0;
const HUGE: f32 = 4096.0;
const MAX_RATIO: f64 = 2.0;

fn blurred(sigma: f32) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::block()
                .fixed_size(SIDE as f32, SIDE as f32)
                .padding(Space::raw(50.0, "test fixture")),
        )
        .bg(Rgba8 {
            r: 255,
            g: 255,
            b: 255,
            a: 255,
        })
        .child(
            container()
                .layout(LayoutStyle::block().fixed_size(SIDE as f32 - 100.0, SIDE as f32 - 100.0))
                .bg(Rgba8 {
                    r: 0,
                    g: 0,
                    b: 0,
                    a: 255,
                })
                .blur(sigma),
        )
}

fn frame_time(sigma: f32, side: u32) -> Option<Duration> {
    let mut surface = Harness::new(move || blurred(sigma)).gpu(side, side, 1.0);
    let started = Instant::now();
    let frame = surface.image().expect("gpu frame");
    frame.map(|_| started.elapsed())
}

#[test]
fn a_huge_blur_costs_no_more_than_a_capped_one() {
    if frame_time(8.0, 256).is_none() {
        return;
    }
    let capped = frame_time(CAPPED, SIDE).expect("capped frame");
    let huge = frame_time(HUGE, SIDE).expect("huge frame");
    let ratio = huge.as_secs_f64() / capped.as_secs_f64();
    assert!(
        ratio < MAX_RATIO,
        "blur({HUGE}) took {huge:?}, {ratio:.2}x blur({CAPPED}) at {capped:?}"
    );
}
