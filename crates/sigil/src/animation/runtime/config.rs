use crate::animation::{Easing, Spring};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum AnimationConfig {
    Spring(Spring),
    Tween {
        ms: u16,
        easing: Easing,
        delay_ms: u16,
    },
    Playback {
        end: Option<f64>,
        cycle: f64,
        delay: f64,
    },
}
