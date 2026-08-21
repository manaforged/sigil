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
use sigil_style::scale::{ColorScale, ScaleRole};
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
///
/// V1 limitation — runtime-registered tokens (those added via
/// [`Theme::register_token`] / [`Theme::with_token`]) are NOT in the
/// snapshot and resolve to `missing_token_fallback` during
/// transitions. Static (non-animated) rendering of the same token
/// goes through `Theme::resolve_color` and works correctly. Carrying
/// the runtime `HashMap` here would require either dropping `Copy`
/// on this snapshot or sharing it via `Rc`; tracked as a follow-up.
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
    // Scales for `<role>-<step>` token resolution (e.g. `accent-9`,
    // `success-3`, `error-11`). Without these, animated transitions
    // through scale tokens fell to `missing_token_fallback`
    // (adversarial review 2026-05-21).
    pub(crate) gray: ColorScale,
    pub(crate) accent_scale: ColorScale,
    pub(crate) success: ColorScale,
    pub(crate) warn: ColorScale,
    pub(crate) error: ColorScale,
    pub(crate) info: ColorScale,
    pub(crate) on_solid: Rgba8,
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
            gray: theme.gray,
            accent_scale: theme.accent_scale,
            success: theme.success,
            warn: theme.warn,
            error: theme.error,
            info: theme.info,
            on_solid: theme.on_solid,
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
            "on-solid" => self.on_solid,
            // Semantic aliases that `Theme::resolve_color` handles.
            "surface-subtle" => self.gray.step(3),
            "on-surface-muted" => self.gray.step(11),
            _ => self
                .scale_step(token)
                .unwrap_or(self.missing_token_fallback),
        }
    }

    /// Parse `<role>-<step>` and return the corresponding scale step,
    /// matching `Theme::scale_step_color`. Returns `None` when the
    /// token does not fit the pattern.
    fn scale_step(self, token: &str) -> Option<Rgba8> {
        let (role_name, step_str) = token.rsplit_once('-')?;
        let role = ScaleRole::from_token(role_name)?;
        let step: u8 = step_str.parse().ok()?;
        if !(1..=12).contains(&step) {
            return None;
        }
        let scale = self.scale(role);
        Some(scale.step(step))
    }

    const fn scale(self, role: ScaleRole) -> ColorScale {
        match role {
            ScaleRole::Gray => self.gray,
            ScaleRole::Accent => self.accent_scale,
            ScaleRole::Success => self.success,
            ScaleRole::Warn => self.warn,
            ScaleRole::Error => self.error,
            ScaleRole::Info => self.info,
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

fn resolve_color_atom(color: sigil_style::ColorAtom, theme: &ThemeSnapshot) -> Rgba8 {
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
#[path = "binding_animate_color_tests.rs"]
mod tests;
