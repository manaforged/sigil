#![allow(clippy::unwrap_used)]
#![allow(clippy::cast_precision_loss, reason = "call counts are small")]

use std::time::Duration;

use sigil::animation::{Keyframes, Property};
use sigil::prelude::*;
use sigil::reactive::Owner;

const CALLS: u32 = 1_000;
const BUDGET: i64 = 4_096;

fn fade(opacity: f32) -> Keyframes {
    Keyframes::new(Duration::from_millis(300))
        .keyframe(0.0, Property::Opacity(opacity))
        .keyframe(1.0, Property::TranslateX(4.0))
}

fn animate(keyframes: Keyframes) {
    drop(container().animation(keyframes));
}

fn retained(warm: u32, mut keyframes: impl FnMut(u32) -> Keyframes) -> i64 {
    let _keyframes = crate::serial::keyframes_alone();
    for call in 0..warm {
        animate(keyframes(call));
    }
    allocation_counter::measure(|| {
        for call in warm..warm + CALLS {
            animate(keyframes(call));
        }
    })
    .bytes_current
}

#[test]
fn equal_finite_keyframes_share_one_entry() {
    let bytes = retained(1, |_| fade(0.5));
    assert!(bytes < BUDGET, "1000 equal keyframes kept {bytes} bytes");
}

#[test]
fn equal_nan_keyframes_share_one_entry() {
    let bytes = retained(1, |_| fade(f32::NAN));
    assert!(
        bytes < BUDGET,
        "1000 equal NaN keyframes kept {bytes} bytes"
    );
}

#[test]
fn keyframes_built_from_changing_values_stay_bounded() {
    let bytes = retained(CALLS, |call| fade(call as f32 / 4_096.0));
    assert!(
        bytes < BUDGET,
        "1000 distinct keyframes kept {bytes} bytes after warm-up"
    );
}

#[test]
fn keyframes_pinned_by_disposed_scopes_are_released() {
    let bytes = retained(CALLS, |call| {
        let keyframes = fade(0.25 + call as f32 / 8_192.0);
        Owner::scope("pinned-animation", || {
            drop(container().animation(keyframes.clone()));
        });
        keyframes
    });
    assert!(
        bytes < BUDGET,
        "1000 scoped distinct keyframes kept {bytes} bytes after warm-up"
    );
}
