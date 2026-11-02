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
pub(crate) const fn config(id: TransitionId) -> Option<AnimationConfig> {
    let preset = match id {
        presets::SMOOTH => SpringPreset::Smooth,
        presets::GENTLE => SpringPreset::Gentle,
        presets::SNAPPY => SpringPreset::Snappy,
        presets::BOUNCY => SpringPreset::Bouncy,
        presets::STIFF => SpringPreset::Stiff,
        presets::FAST => return Some(cubic_in_out(120)),
        presets::DEFAULT => return Some(cubic_in_out(200)),
        presets::SLOW => return Some(cubic_in_out(320)),
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
