#![allow(clippy::redundant_pub_crate)]

#[cfg(test)]
use criterion as _;

pub(crate) mod animator;
pub(crate) mod binding_animate;
pub(crate) mod binding_animate_color;
pub(crate) mod clock;
pub(crate) mod config;
pub(crate) mod motion;
pub(crate) mod playback;
pub(crate) mod registry;
pub(crate) mod state;
pub(crate) mod style;
pub(crate) mod targets;

pub use animator::Animator;
pub use binding_animate_color::linear_channel_to_srgb;
#[cfg(test)]
pub(crate) use clock::tick_clocks;
pub use clock::{clock, running_clocks};
pub(crate) use config::AnimationConfig;
#[cfg(feature = "kittest")]
#[doc(hidden)]
pub use motion::MotionPrefs;
#[cfg(not(feature = "kittest"))]
pub(crate) use motion::MotionPrefs;
pub use motion::reduced_motion;
#[cfg(test)]
pub(crate) use registry::config;
pub use registry::presets;
pub use state::{active_count, tick};
