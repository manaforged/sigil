use crate::reactive::signal;
use crate::style::{ColorAtomKind, Rgba8};

use crate::animation_runtime::{AnimationConfig, Animator};

use super::binding_animate::AttachedColorAnimator;

#[must_use]
pub fn srgb_channel_to_linear(c: u8) -> f32 {
    crate::style::srgb::decode(c)
}

#[must_use]
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "value is clamped to 0..=255 before the cast — bounded"
)]
pub fn linear_channel_to_srgb(v: f32) -> u8 {
    (crate::style::srgb::encode(v) * 255.0)
        .round()
        .clamp(0.0, 255.0) as u8
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "clamped to 0..=255 before cast"
)]
const fn alpha_byte(value: f32) -> u8 {
    value.round().clamp(0.0, 255.0) as u8
}

#[must_use]
pub(crate) fn read_color(entry: &AttachedColorAnimator) -> Rgba8 {
    let [r, g, b, a] = entry.channels.map(|channel| channel.peek());
    Rgba8 {
        r: linear_channel_to_srgb(r),
        g: linear_channel_to_srgb(g),
        b: linear_channel_to_srgb(b),
        a: alpha_byte(a),
    }
}

pub(crate) fn build_color_entry(
    kind: ColorAtomKind,
    initial: Rgba8,
    config: AnimationConfig,
) -> AttachedColorAnimator {
    let channels = linear_channels(initial).map(signal);
    AttachedColorAnimator {
        kind,
        channels,
        animators: channels.map(|channel| Animator::new(channel, config)),
        last_target: initial,
    }
}

pub(crate) fn retarget_color_entry(entry: &mut AttachedColorAnimator, target: Rgba8) {
    if entry.last_target == target {
        return;
    }
    entry.last_target = target;
    for (animator, value) in entry.animators.iter_mut().zip(linear_channels(target)) {
        animator.target(value);
    }
}

fn linear_channels(color: Rgba8) -> [f32; 4] {
    [
        srgb_channel_to_linear(color.r),
        srgb_channel_to_linear(color.g),
        srgb_channel_to_linear(color.b),
        f32::from(color.a),
    ]
}

#[cfg(test)]
#[path = "binding_animate_color_tests.rs"]
mod tests;
