use std::sync::Arc;

use crate::reactive::signal;
use crate::style::{ColorAtom, ColorAtomKind, ColorKey, Rgba8, StyleAtom, Theme};

use crate::animation_runtime::{AnimationConfig, Animator};

use super::binding_animate::AttachedColorAnimator;

#[derive(Clone, Debug)]
pub(crate) struct ThemeSnapshot(Arc<Theme>);

impl ThemeSnapshot {
    pub(crate) fn from_theme(theme: &Theme) -> Self {
        Self(Arc::new(theme.clone()))
    }

    pub(crate) fn resolve(&self, key: ColorKey) -> Rgba8 {
        self.0.resolve_color(key)
    }
}

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

#[must_use]
pub fn lerp_rgba8(a: Rgba8, b: Rgba8, t: f32) -> Rgba8 {
    let t = t.clamp(0.0, 1.0);
    let lerp_ch = |from: u8, to: u8| {
        let from = srgb_channel_to_linear(from);
        linear_channel_to_srgb((srgb_channel_to_linear(to) - from).mul_add(t, from))
    };
    Rgba8 {
        r: lerp_ch(a.r, b.r),
        g: lerp_ch(a.g, b.g),
        b: lerp_ch(a.b, b.b),
        a: alpha_byte((f32::from(b.a) - f32::from(a.a)).mul_add(t, f32::from(a.a))),
    }
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

#[must_use]
pub(crate) fn find_color(
    kind: ColorAtomKind,
    atoms: &[StyleAtom],
    theme: &ThemeSnapshot,
) -> Option<Rgba8> {
    atoms.iter().rev().find_map(|atom| match (*atom, kind) {
        (StyleAtom::BackgroundColor(color), ColorAtomKind::Background)
        | (StyleAtom::TextColor(color), ColorAtomKind::Text) => {
            Some(resolve_color_atom(color, theme))
        }
        (StyleAtom::BackgroundThemeRef(token), ColorAtomKind::Background)
        | (StyleAtom::TextThemeRef(token), ColorAtomKind::Text) => Some(theme.resolve(token)),
        _ => None,
    })
}

fn resolve_color_atom(color: ColorAtom, theme: &ThemeSnapshot) -> Rgba8 {
    match color {
        ColorAtom::Rgba8(rgba) => rgba,
        ColorAtom::ThemeRef(token) => theme.resolve(token),
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
