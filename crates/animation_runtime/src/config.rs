//! `AnimationConfig` — the type the [`AnimationRegistry`] returns. Names
//! a concrete spring shape or a duration tween.

use sigil_animation::{Easing, SpringPreset};

/// Either a spring preset or a duration tween. Re-targetable after
/// construction. Returned by [`crate::AnimationRegistry::get`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnimationConfig {
    /// Hooke's-law spring with a named preset.
    Spring {
        /// Preset shape (stiffness/damping pair).
        preset: SpringPreset,
    },
    /// Fixed-duration tween under an easing curve.
    Tween {
        /// Duration in milliseconds.
        ms: u16,
        /// Easing curve applied to normalised `t`.
        easing: Easing,
    },
}
