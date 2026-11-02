#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]

use std::cell::Cell;
use std::rc::Rc;

use sigil::prelude::*;
use sigil_kittest::{Harness, InteractionMode};

#[test]
fn a_capture_publishes_its_viewport_like_a_host() {
    let seen = Rc::new(Cell::new(0.0_f32));
    let probe = Rc::clone(&seen);
    let _ = Harness::render(text(move || {
        probe.set(sigil::app::viewport_size().get().width);
        String::new()
    }))
    .stub()
    .mode(InteractionMode::Default)
    .size([320, 200])
    .hit((0.0, 0.0))
    .image();
    assert!(
        (seen.get() - 320.0).abs() < f32::EPSILON,
        "viewport width seen: {}",
        seen.get()
    );
}
