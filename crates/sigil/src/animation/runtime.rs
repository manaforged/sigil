#![allow(clippy::redundant_pub_crate)]

#[cfg(test)]
use criterion as _;

pub(crate) mod animator;
pub(crate) mod binding_animate;
pub(crate) mod binding_animate_color;
pub(crate) mod clock;
pub(crate) mod config;
pub(crate) mod motion_prefs;
pub(crate) mod playback;
pub(crate) mod registry;
pub(crate) mod state;
pub(crate) mod style;
pub(crate) mod targets;

pub use animator::Animator;
pub(crate) use binding_animate_color::linear_channel_to_srgb;
pub use clock::running_clocks;
#[cfg(test)]
pub(crate) use clock::tick_clocks;
pub(crate) use config::AnimationConfig;
#[cfg(feature = "kittest")]
#[doc(hidden)]
pub use motion_prefs::MotionPrefs;
#[cfg(not(feature = "kittest"))]
pub(crate) use motion_prefs::MotionPrefs;
pub use motion_prefs::reduced_motion;
#[cfg(test)]
pub(crate) use registry::config;
pub use registry::presets;
pub use state::{active_count, tick};
