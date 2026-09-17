use crate::animation::{Easing, SpringPreset};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AnimationConfig {
    Spring { preset: SpringPreset },
    Tween { ms: u16, easing: Easing },
}
