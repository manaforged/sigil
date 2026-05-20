//! Colour-channel animation — W5.1c.
//!
//! Phase 2 of DR-053 §5: extends the W5.1a binding-install hook with
//! colour animators driving [`Rgba8`] outputs (background / text).
//!
//! **Colour-space contract.** sRGB-encoded RGB channels are converted
//! to linear-light `f32` at install time + retarget. The per-channel
//! animator interpolates in linear-light. The reader composes the four
//! signals back to an [`Rgba8`] via the inverse sRGB encode at read
//! time. Alpha interpolates linearly — alpha is not gamma-encoded.
//!
//! Why linear-light: the renderer's `Rgba8UnormSrgb` swapchain format
//! applies the inverse sRGB transfer to the colour values it samples,
//! so a perceptually-linear hover transition must be linear in light,
//! not in sRGB. (Confirmed against the RC1 fix in `4a6e12e` — same
//! contract the static palette uses.)

use sigil_reactive::signal;
use sigil_style::{ColorAtomKind, Rgba8, StyleAtom, Theme};

use crate::{AnimationConfig, Animator};

use super::binding_animate::AttachedColorAnimator;

/// Snapshot of the subset of [`Theme`] colour tokens the colour-animator
/// hook needs to resolve `*ThemeRef` atoms at retarget time.
///
/// Stored inside [`super::binding_animate::AttachedAnimators`] so a
/// later `retarget_to_slot` call can resolve token references that the
/// state-slot overlay introduces without re-borrowing the kernel's
/// theme.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ThemeSnapshot {
    pub(crate) accent: Rgba8,
    pub(crate) accent_hover: Rgba8,
    pub(crate) on_accent: Rgba8,
    pub(crate) surface: Rgba8,
    pub(crate) surface_hover: Rgba8,
    pub(crate) surface_pressed: Rgba8,
    pub(crate) surface_pressed_strong: Rgba8,
    pub(crate) on_surface: Rgba8,
    pub(crate) border: Rgba8,
    pub(crate) missing_token_fallback: Rgba8,
}

impl ThemeSnapshot {
    pub(crate) const fn from_theme(theme: &Theme) -> Self {
        Self {
            accent: theme.accent,
            accent_hover: theme.accent_hover,
            on_accent: theme.on_accent,
            surface: theme.surface,
            surface_hover: theme.surface_hover,
            surface_pressed: theme.surface_pressed,
            surface_pressed_strong: theme.surface_pressed_strong,
            on_surface: theme.on_surface,
            border: theme.border,
            missing_token_fallback: theme.missing_token_fallback,
        }
    }

    pub(crate) fn resolve(self, token: &str) -> Rgba8 {
        match token {
            "accent" => self.accent,
            "accent-hover" => self.accent_hover,
            "on-accent" => self.on_accent,
            "surface" => self.surface,
            "surface-hover" => self.surface_hover,
            "surface-pressed" => self.surface_pressed,
            "surface-pressed-strong" => self.surface_pressed_strong,
            "on-surface" => self.on_surface,
            "border" => self.border,
            _ => self.missing_token_fallback,
        }
    }
}

/// sRGB transfer (channel → linear-light). Matches the IEC 61966-2-1
/// piecewise definition the rest of the workspace uses.
#[must_use]
pub fn srgb_channel_to_linear(c: u8) -> f32 {
    let v = f32::from(c) / 255.0;
    if v <= 0.040_45 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}

/// Inverse sRGB transfer (linear-light → channel).
#[must_use]
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "value is clamped to 0..=255 before the cast — bounded"
)]
pub fn linear_channel_to_srgb(v: f32) -> u8 {
    let v = v.clamp(0.0, 1.0);
    let s = if v <= 0.003_130_8 {
        v * 12.92
    } else {
        1.055_f32.mul_add(v.powf(1.0 / 2.4), -0.055)
    };
    (s * 255.0).round().clamp(0.0, 255.0) as u8
}

/// Interpolate two `Rgba8` colours in linear-light space.
///
/// `t` is clamped to `[0.0, 1.0]`. RGB channels are converted to
/// linear, lerped, then converted back. Alpha is lerped linearly.
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

/// Read the current driven colour out of a [`AttachedColorAnimator`].
///
/// Composes the four channel signals back to an `Rgba8` via the inverse
/// sRGB encode.
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

/// Resolve a colour-bearing atom in `atoms` (last writer wins). When
/// the atom is a `*ThemeRef`, resolve it via `theme`.
#[must_use]
pub(crate) fn find_color(
    kind: ColorAtomKind,
    atoms: &[StyleAtom],
    theme: ThemeSnapshot,
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

fn resolve_color_atom(color: sigil_style::ColorAtom, theme: ThemeSnapshot) -> Rgba8 {
    match color {
        sigil_style::ColorAtom::Rgba8(rgba) => rgba,
        sigil_style::ColorAtom::ThemeRef(token) => theme.resolve(token),
    }
}

/// Build a fresh colour animator entry. RGB initial values are written
/// in linear-light; alpha sits in `[0, 255]` so reads convert back
/// directly.
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

/// Retarget every animator in `entry` toward `target`. Skips when the
/// resolved colour matches the last target (no-op gate).
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
mod tests {
    #![allow(clippy::unwrap_used, clippy::panic)]

    use super::*;

    fn rgba(r: u8, g: u8, b: u8, a: u8) -> Rgba8 {
        Rgba8 { r, g, b, a }
    }

    #[test]
    fn srgb_roundtrip_at_extremes() {
        for c in [0u8, 1, 64, 128, 200, 254, 255] {
            let lin = srgb_channel_to_linear(c);
            let back = linear_channel_to_srgb(lin);
            assert!(
                back.abs_diff(c) <= 1,
                "sRGB roundtrip drift > 1 LSB for {c}: lin={lin}, back={back}",
            );
        }
    }

    #[test]
    fn lerp_endpoint_t0_equals_a() {
        let a = rgba(64, 128, 200, 255);
        let b = rgba(200, 64, 32, 128);
        let mixed = lerp_rgba8(a, b, 0.0);
        assert_eq!(mixed, a, "t=0 must equal endpoint a");
    }

    #[test]
    fn lerp_endpoint_t1_equals_b() {
        let a = rgba(64, 128, 200, 255);
        let b = rgba(200, 64, 32, 128);
        let mixed = lerp_rgba8(a, b, 1.0);
        assert_eq!(mixed, b, "t=1 must equal endpoint b");
    }

    #[test]
    fn lerp_midpoint_between_endpoints() {
        // Midpoint of (64, 128, 200) and (200, 64, 32) is between the
        // endpoints per-channel — linear-light interpolation tolerates
        // a ~2 LSB drift after the sRGB roundtrip.
        let a = rgba(64, 128, 200, 255);
        let b = rgba(200, 64, 32, 128);
        let mid = lerp_rgba8(a, b, 0.5);
        // Each channel sits strictly between the endpoint values (or
        // exactly equal when endpoints coincide).
        assert!(mid.r >= a.r.min(b.r) && mid.r <= a.r.max(b.r));
        assert!(mid.g >= a.g.min(b.g) && mid.g <= a.g.max(b.g));
        assert!(mid.b >= a.b.min(b.b) && mid.b <= a.b.max(b.b));
        // Alpha midpoint of 255 and 128 = 192 ± 1.
        assert!(mid.a.abs_diff(192) <= 1, "alpha midpoint drift: {}", mid.a);
    }

    #[test]
    fn lerp_t_clamps_outside_unit_interval() {
        let a = rgba(0, 0, 0, 0);
        let b = rgba(255, 255, 255, 255);
        assert_eq!(lerp_rgba8(a, b, -0.5), a, "t<0 clamps to 0");
        assert_eq!(lerp_rgba8(a, b, 1.5), b, "t>1 clamps to 1");
    }

    #[test]
    fn lerp_linear_space_is_brighter_than_srgb_at_midpoint() {
        // Linear-light midpoint of black and white (which is sRGB
        // 188) is visibly brighter than the naive sRGB midpoint (128).
        // This is the contract the renderer relies on — verifying it
        // catches accidental sRGB-space lerp regressions.
        let mid = lerp_rgba8(rgba(0, 0, 0, 255), rgba(255, 255, 255, 255), 0.5);
        assert!(
            mid.r >= 180 && mid.r <= 195,
            "linear-light midpoint of black+white must sit near 188 sRGB (got {})",
            mid.r,
        );
    }
}
