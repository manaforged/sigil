use sigil_animation::{Easing, SpringPreset};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnimationConfig {
    Spring { preset: SpringPreset },
    Tween { ms: u16, easing: Easing },
}
