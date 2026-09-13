#![allow(clippy::redundant_pub_crate)]

#[cfg(test)]
use criterion as _;

pub(crate) mod animator;
pub mod binding_animate;
pub(crate) mod binding_animate_color;
pub(crate) mod config;
pub(crate) mod motion;
pub(crate) mod registry;
pub(crate) mod state;

pub use crate::style::NumericAtomKind;
pub use animator::{Animator, AnimatorStatus};
pub use binding_animate::{
    ANIMATION_REGISTRY_MISS, AttachedAnimator, AttachedAnimators, AttachedColorAnimator,
    install_animators, install_animators_themed,
};
pub use binding_animate_color::{lerp_rgba8, linear_channel_to_srgb, srgb_channel_to_linear};
pub use config::AnimationConfig;
pub use motion::MotionPrefs;
pub use registry::{config, presets};
pub use state::{active_count, tick};
