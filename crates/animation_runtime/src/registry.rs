//! Transition id → [`AnimationConfig`].
//!
//! Eight defaults ship per DR-053 §4 — five springs (`Smooth` /
//! `Gentle` / `Snappy` / `Bouncy` / `Stiff` at IDs `0..=4`) and three
//! `CubicInOut` tweens (`fast 120 ms` / `default 200 ms` /
//! `slow 320 ms` at IDs `16..=18`).
//!
//! At eight entries the lookup is a `match`, not a `phf` table — a
//! branch tree is smaller and faster than the hash lookup at this
//! scale, and the macro feature gate would otherwise pull `phf_macros`
//! into the build for no measurable win.

use sigil_animation::{Easing, SpringPreset};
use sigil_style::TransitionId;

use crate::config::AnimationConfig;

/// Default transition IDs published by the registry.
pub mod presets {
    use sigil_style::TransitionId;

    /// `Smooth` spring — default desktop UI feel.
    pub const SMOOTH: TransitionId = TransitionId(0);
    /// `Gentle` spring — loose, exploratory.
    pub const GENTLE: TransitionId = TransitionId(1);
    /// `Snappy` spring — click feedback.
    pub const SNAPPY: TransitionId = TransitionId(2);
    /// `Bouncy` spring — playful overshoot.
    pub const BOUNCY: TransitionId = TransitionId(3);
    /// `Stiff` spring — near-instant.
    pub const STIFF: TransitionId = TransitionId(4);
    /// 120 ms cubic-in-out tween.
    pub const FAST: TransitionId = TransitionId(16);
    /// 200 ms cubic-in-out tween (default duration).
    pub const DEFAULT: TransitionId = TransitionId(17);
    /// 320 ms cubic-in-out tween.
    pub const SLOW: TransitionId = TransitionId(18);
}

/// Config for a shipped transition id.
///
/// Returns `None` when `id` is not one of the eight presets — the
/// binding-install hook logs a merge warning and applies instantly.
#[must_use]
pub const fn config(id: TransitionId) -> Option<AnimationConfig> {
    Some(match id.0 {
        0 => AnimationConfig::Spring {
            preset: SpringPreset::Smooth,
        },
        1 => AnimationConfig::Spring {
            preset: SpringPreset::Gentle,
        },
        2 => AnimationConfig::Spring {
            preset: SpringPreset::Snappy,
        },
        3 => AnimationConfig::Spring {
            preset: SpringPreset::Bouncy,
        },
        4 => AnimationConfig::Spring {
            preset: SpringPreset::Stiff,
        },
        16 => AnimationConfig::Tween {
            ms: 120,
            easing: Easing::CubicInOut,
        },
        17 => AnimationConfig::Tween {
            ms: 200,
            easing: Easing::CubicInOut,
        },
        18 => AnimationConfig::Tween {
            ms: 320,
            easing: Easing::CubicInOut,
        },
        _ => return None,
    })
}
