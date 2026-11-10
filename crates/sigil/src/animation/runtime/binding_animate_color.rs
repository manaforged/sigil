use crate::reactive::signal;
use crate::style::srgb::byte;
use crate::style::{ColorAtomKind, Rgba8};

use crate::animation::runtime::{AnimationConfig, Animator};

use super::binding_animate::AttachedColorAnimator;

#[must_use]
#[cfg_attr(
    not(feature = "scene"),
    allow(dead_code, reason = "the renderer is its caller")
)]
pub(crate) fn linear_channel_to_srgb(v: f32) -> u8 {
    byte(crate::style::srgb::encode(v) * 255.0)
}

fn premultiplied(color: Rgba8) -> [f32; 4] {
    let alpha = f32::from(color.a);
    let channel = |value: u8| f32::from(value) * alpha / 255.0;
    [channel(color.r), channel(color.g), channel(color.b), alpha]
}

fn unpremultiplied([r, g, b, alpha]: [f32; 4]) -> Rgba8 {
    if alpha <= 0.0 {
        return Rgba8 {
            r: 0,
            g: 0,
            b: 0,
            a: 0,
        };
    }
    let channel = |value: f32| byte(value * 255.0 / alpha);
    Rgba8 {
        r: channel(r),
        g: channel(g),
        b: channel(b),
        a: byte(alpha),
    }
}

/// Interpolates two colors in gamma-encoded sRGB with premultiplied alpha, as CSS Color 4 does for legacy colors.
pub(crate) fn mix(from: Rgba8, to: Rgba8, t: f32) -> Rgba8 {
    let (start, end) = (premultiplied(from), premultiplied(to));
    let mut channels = start;
    for (channel, (a, b)) in channels.iter_mut().zip(start.into_iter().zip(end)) {
        *channel = (b - a).mul_add(t, a);
    }
    unpremultiplied(channels)
}

#[must_use]
pub(crate) fn read_color(entry: &AttachedColorAnimator) -> Rgba8 {
    unpremultiplied(entry.channels.map(|channel| channel.peek()))
}

pub(crate) fn build_color_entry(
    kind: ColorAtomKind,
    initial: Rgba8,
    config: AnimationConfig,
) -> AttachedColorAnimator {
    let channels = premultiplied(initial).map(signal);
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
    for (animator, value) in entry.animators.iter_mut().zip(premultiplied(target)) {
        animator.target(value);
    }
}

#[cfg(test)]
#[path = "binding_animate_color_tests.rs"]
mod tests;
