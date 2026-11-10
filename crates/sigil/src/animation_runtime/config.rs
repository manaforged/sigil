use crate::animation::{Easing, SpringPreset};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum AnimationConfig {
    Spring {
        preset: SpringPreset,
    },
    Tween {
        ms: u16,
        easing: Easing,
        delay_ms: u16,
    },
    Playback {
        end: Option<f64>,
        cycle: f64,
    },
}
