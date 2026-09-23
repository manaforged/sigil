//! Animation primitives: easings, keyframes, springs, and tweens.
mod bezier;
mod easing;
mod keyframes;
mod spring;
mod tween;

pub use bezier::CubicBezier;
pub use easing::Easing;
pub use keyframes::{AnimationDirection, AnimationFill, Keyframe, Keyframes};
pub use spring::{Spring, SpringPreset};
pub use tween::Tween;
