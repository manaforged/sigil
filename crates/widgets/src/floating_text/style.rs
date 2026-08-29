use sigil_style::scale::ScaleRole;
use sigil_style::{Rgba8, Theme};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FloaterStyle {
    Damage,
    Heal,
    Miss,
}

impl FloaterStyle {
    const fn scale_step(self) -> (ScaleRole, u8) {
        match self {
            Self::Damage => (ScaleRole::Error, 11),
            Self::Heal => (ScaleRole::Success, 11),
            Self::Miss => (ScaleRole::Gray, 12),
        }
    }

    #[must_use]
    pub fn color(self) -> Rgba8 {
        let (role, step) = self.scale_step();
        Theme::default().scale(role).step(step)
    }
}
