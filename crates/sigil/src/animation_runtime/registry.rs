use crate::animation::{Easing, SpringPreset};
use crate::style::TransitionId;

use crate::animation_runtime::config::AnimationConfig;

pub mod presets {
    use crate::style::TransitionId;

    pub const SMOOTH: TransitionId = TransitionId(0);
    pub const GENTLE: TransitionId = TransitionId(1);
    pub const SNAPPY: TransitionId = TransitionId(2);
    pub const BOUNCY: TransitionId = TransitionId(3);
    pub const STIFF: TransitionId = TransitionId(4);
    pub const FAST: TransitionId = TransitionId(16);
    pub const DEFAULT: TransitionId = TransitionId(17);
    pub const SLOW: TransitionId = TransitionId(18);
}

#[must_use]
pub const fn config(id: TransitionId) -> Option<AnimationConfig> {
    Some(match id.0 {
        0 => AnimationConfig::Spring {
            preset: SpringPreset::Smooth,
        },
        1 => AnimationConfig::Spring {
            preset: SpringPreset::Gentle,
        },
        2 => AnimationConfig::Spring {
            preset: SpringPreset::Snappy,
        },
        3 => AnimationConfig::Spring {
            preset: SpringPreset::Bouncy,
        },
        4 => AnimationConfig::Spring {
            preset: SpringPreset::Stiff,
        },
        16 => AnimationConfig::Tween {
            ms: 120,
            easing: Easing::CubicInOut,
        },
        17 => AnimationConfig::Tween {
            ms: 200,
            easing: Easing::CubicInOut,
        },
        18 => AnimationConfig::Tween {
            ms: 320,
            easing: Easing::CubicInOut,
        },
        _ => return None,
    })
}
