mod easing;
mod keyframes;
mod motion;
#[cfg_attr(
    not(feature = "kittest"),
    allow(unused_imports, reason = "public only under kittest")
)]
pub(crate) mod runtime;
mod spring;
mod transition;
mod tween;

pub use easing::Easing;
pub use keyframes::{Keyframes, Property};
kittest_pub! {
    #[doc(hidden)]
    pub use keyframes::KeyframesId;
}
pub(crate) use motion::Motion;
pub use runtime::clock::clock;
pub use runtime::reduced_motion;
kittest_pub! {
    #[doc(hidden)]
    pub use runtime::{
        Animator, MotionPrefs, active_count, linear_channel_to_srgb, presets, running_clocks, tick,
    };
}
pub(crate) use spring::Spring;
kittest_pub! {
    #[doc(hidden)]
    pub use spring::SpringPreset;
}
pub use transition::Transition;
pub(crate) use tween::Tween;
