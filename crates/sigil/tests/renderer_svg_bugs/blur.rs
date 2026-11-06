use std::time::{Duration, Instant};

use sigil::kernel::container;
use sigil::layout::LayoutStyle;
use sigil::prelude::*;
use sigil_kittest::Harness;

const LIMIT: Duration = Duration::from_secs(3);
const SIDE: u32 = 4000;

fn blurred() -> NodeBuilder {
    container()
        .layout(LayoutStyle::block().fixed_size(SIDE as f32, SIDE as f32).padding(50.0))
        .bg(Rgba8 { r: 255, g: 255, b: 255, a: 255 })
        .child(
            container()
                .layout(LayoutStyle::block().fixed_size(SIDE as f32 - 100.0, SIDE as f32 - 100.0))
                .bg(Rgba8 { r: 0, g: 0, b: 0, a: 255 })
                .blur(4096.0),
        )
}

#[test]
fn a_huge_blur_renders_in_bounded_time() {
    let mut surface = Harness::new(blurred).gpu(SIDE, SIDE, 1.0);
    let started = Instant::now();
    let frame = surface.image().expect("gpu frame");
    let elapsed = started.elapsed();
    assert!(frame.is_some() || elapsed < LIMIT, "no frame");
    assert!(elapsed < LIMIT, "a blur(4096) frame took {elapsed:?}");
}
