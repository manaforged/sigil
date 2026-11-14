#![allow(clippy::redundant_pub_crate)]

#[cfg(test)]
use criterion as _;

pub(crate) mod animator;
pub(crate) mod binding_animate;
pub(crate) mod binding_animate_color;
pub(crate) mod clock;
pub(crate) mod config;
pub(crate) mod layout_motion;
pub(crate) mod motion_prefs;
pub(crate) mod playback;
pub(crate) mod registry;
pub(crate) mod shared;
pub(crate) mod state;
pub(crate) mod style;
pub(crate) mod targets;

pub use animator::Animator;
pub(crate) use animator::SignalAnimator;
pub(crate) use binding_animate_color::linear_channel_to_srgb;
kittest_pub! {
    #[doc(hidden)]
    pub use clock::running_clocks;
}
#[cfg(test)]
pub(crate) use clock::tick_clocks;
pub(crate) use config::AnimationConfig;
#[cfg(feature = "kittest")]
#[doc(hidden)]
pub use motion_prefs::MotionPrefs;
#[cfg(not(feature = "kittest"))]
pub(crate) use motion_prefs::MotionPrefs;
pub use motion_prefs::{reduced_motion, reduced_transparency};
#[cfg(test)]
pub(crate) use registry::config;
kittest_pub! {
    #[doc(hidden)]
    pub use registry::presets;
}
kittest_pub! {
    #[doc(hidden)]
    pub use state::{active_count, tick};
}

pub(crate) fn animating() -> bool {
    state::active_count() > 0 || clock::running_clocks() > 0
}
