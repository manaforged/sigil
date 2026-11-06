mod easing;
mod keyframes;
mod motion;
mod spring;
mod tween;

pub use crate::animation_runtime::clock::clock;
pub use crate::animation_runtime::reduced_motion;
pub use easing::Easing;
pub use keyframes::{Keyframes, Property};
pub(crate) use motion::Motion;
pub(crate) use spring::Spring;
kittest_pub! {
    #[doc(hidden)]
    pub use spring::SpringPreset;
}
pub(crate) use tween::Tween;
