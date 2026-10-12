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
    let preset = match id.0 {
        0 => SpringPreset::Smooth,
        1 => SpringPreset::Gentle,
        2 => SpringPreset::Snappy,
        3 => SpringPreset::Bouncy,
        4 => SpringPreset::Stiff,
        16 => return Some(cubic_in_out(120)),
        17 => return Some(cubic_in_out(200)),
        18 => return Some(cubic_in_out(320)),
        _ => return None,
    };
    Some(AnimationConfig::Spring { preset })
}

const fn cubic_in_out(ms: u16) -> AnimationConfig {
    AnimationConfig::Tween {
        ms,
        easing: Easing::CubicInOut,
    }
}
