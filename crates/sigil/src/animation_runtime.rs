#![allow(clippy::redundant_pub_crate)]

#[cfg(test)]
use criterion as _;

pub(crate) mod animator;
pub(crate) mod binding_animate;
pub(crate) mod binding_animate_color;
pub(crate) mod config;
pub(crate) mod keyframes;
pub(crate) mod motion;
pub(crate) mod motion_state;
pub(crate) mod playback;
pub(crate) mod registry;
pub(crate) mod state;
pub(crate) mod style;

pub use animator::{Animator, AnimatorStatus};
#[cfg(any(test, feature = "kittest"))]
pub use binding_animate::ANIMATION_REGISTRY_MISS;
pub use binding_animate::{
    AttachedAnimator, AttachedAnimators, AttachedColorAnimator, install_animators,
    install_animators_themed,
};
pub use binding_animate_color::{lerp_rgba8, linear_channel_to_srgb, srgb_channel_to_linear};
pub use config::AnimationConfig;
pub use motion::MotionPrefs;
pub use playback::Playback;
pub use registry::{config, presets};
pub use state::{active_count, tick};
