use std::sync::Arc;

use crate::reactive::signal;
use crate::style::{ColorAtomKind, ColorKey, Rgba8, StyleAtom, Theme};

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
    let lerp_ch = |a_u8: u8, b_u8: u8| -> u8 {
        let a_lin = srgb_channel_to_linear(a_u8);
        let b_lin = srgb_channel_to_linear(b_u8);
        let mixed = (b_lin - a_lin).mul_add(t, a_lin);
        linear_channel_to_srgb(mixed)
    };
    let alpha = {
        let mixed = (f32::from(b.a) - f32::from(a.a)).mul_add(t, f32::from(a.a));
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "clamped to 0..=255 before cast"
        )]
        {
            mixed.round().clamp(0.0, 255.0) as u8
        }
    };
    Rgba8 {
        r: lerp_ch(a.r, b.r),
        g: lerp_ch(a.g, b.g),
        b: lerp_ch(a.b, b.b),
        a: alpha,
    }
}

#[must_use]
pub(crate) fn read_color(entry: &AttachedColorAnimator) -> Rgba8 {
    let r = linear_channel_to_srgb(entry.channels[0].peek());
    let g = linear_channel_to_srgb(entry.channels[1].peek());
    let b = linear_channel_to_srgb(entry.channels[2].peek());
    let a = {
        let v = entry.channels[3].peek().clamp(0.0, 255.0);
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "value clamped to 0..=255 before cast"
        )]
        {
            v.round() as u8
        }
    };
    Rgba8 { r, g, b, a }
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

fn resolve_color_atom(color: crate::style::ColorAtom, theme: &ThemeSnapshot) -> Rgba8 {
    match color {
        crate::style::ColorAtom::Rgba8(rgba) => rgba,
        crate::style::ColorAtom::ThemeRef(token) => theme.resolve(token),
    }
}

pub(crate) fn build_color_entry(
    kind: ColorAtomKind,
    initial: Rgba8,
    config: AnimationConfig,
) -> AttachedColorAnimator {
    let r = signal(srgb_channel_to_linear(initial.r));
    let g = signal(srgb_channel_to_linear(initial.g));
    let b = signal(srgb_channel_to_linear(initial.b));
    let a = signal(f32::from(initial.a));
    AttachedColorAnimator {
        kind,
        channels: [r, g, b, a],
        animators: [
            Animator::new(r, config),
            Animator::new(g, config),
            Animator::new(b, config),
            Animator::new(a, config),
        ],
        last_target: initial,
    }
}

pub(crate) fn retarget_color_entry(entry: &mut AttachedColorAnimator, target: Rgba8) {
    if entry.last_target == target {
        return;
    }
    entry.last_target = target;
    entry.animators[0].target(srgb_channel_to_linear(target.r));
    entry.animators[1].target(srgb_channel_to_linear(target.g));
    entry.animators[2].target(srgb_channel_to_linear(target.b));
    entry.animators[3].target(f32::from(target.a));
}

#[cfg(test)]
#[path = "binding_animate_color_tests.rs"]
mod tests;
