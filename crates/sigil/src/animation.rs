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
kittest_pub! {
    #[doc(hidden)]
    pub use keyframes::KeyframesId;
}
pub use keyframes::{Keyframes, Property};
pub(crate) use motion::Motion;
pub use runtime::clock::{clock, every_frame};
pub use runtime::{Animator, reduced_motion};
kittest_pub! {
    #[doc(hidden)]
    pub use runtime::{MotionPrefs, active_count, presets, running_clocks, tick};
}
pub use spring::Spring;
pub(crate) use spring::SpringState;
pub use transition::Transition;
pub(crate) use tween::Tween;
