use crate::style::{ControlSize, Elevation, Px, SPACING_STEP, StyleAtom};

pub const BORDER_WIDTH: Px = Px(1);
pub const DISABLED_OPACITY: u16 = 5_000;
pub const ICON_SIZE: Px = Px(20);
pub const OVERLAY_SHADOW: Elevation = Elevation::Md;

#[allow(
    clippy::cast_lossless,
    reason = "u8 to f32 is exact; From is not const"
)]
#[must_use]
pub const fn space(steps: u8) -> f32 {
    SPACING_STEP * steps as f32
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Radius {
    None,
    Sm,
    Base,
    Md,
    Lg,
    Full,
}

impl Radius {
    #[must_use]
    pub(crate) const fn atom(self) -> StyleAtom {
        StyleAtom::Radius(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Metrics {
    cell: Option<f32>,
}

impl Metrics {
    #[must_use]
    pub fn current() -> Self {
        Self {
            cell: crate::style::cell_size(),
        }
    }

    #[must_use]
    pub const fn window() -> Self {
        Self { cell: None }
    }

    #[must_use]
    pub const fn is_grid(self) -> bool {
        self.cell.is_some()
    }

    #[must_use]
    pub fn control_height(self, size: ControlSize) -> f32 {
        self.cell_or(match size {
            ControlSize::Compact => space(5) + 2.0,
            ControlSize::Default => space(7),
            ControlSize::Wide | ControlSize::Grow => space(9),
        })
    }

    #[must_use]
    pub const fn control_width(self, size: ControlSize) -> f32 {
        match size {
            ControlSize::Compact => space(6),
            ControlSize::Default => space(8),
            ControlSize::Wide | ControlSize::Grow => space(37),
        }
    }

    #[must_use]
    pub fn padding(self, size: ControlSize) -> f32 {
        self.cell_or(match size {
            ControlSize::Compact => space(1) + 2.0,
            ControlSize::Default => space(2) + 2.0,
            ControlSize::Wide | ControlSize::Grow => space(3) + 2.0,
        })
    }

    #[must_use]
    pub fn gap(self, size: ControlSize) -> f32 {
        self.cell_or(match size {
            ControlSize::Compact => space(1),
            ControlSize::Default => space(2),
            ControlSize::Wide | ControlSize::Grow => space(3),
        })
    }

    #[must_use]
    pub const fn radius(self, radius: Radius) -> Px {
        if self.cell.is_some() {
            return Px(0);
        }
        match radius {
            Radius::None => Px(0),
            Radius::Sm => Px(2),
            Radius::Base => Px(4),
            Radius::Md => Px(6),
            Radius::Lg => Px(8),
            Radius::Full => Px(9_999),
        }
    }

    #[must_use]
    pub const fn ring_width(self) -> f32 {
        2.0
    }

    #[must_use]
    pub const fn ring_offset(self) -> f32 {
        2.0
    }

    #[must_use]
    pub fn icon_size(self) -> f32 {
        self.cell_or(f32::from(ICON_SIZE.0))
    }

    #[must_use]
    pub fn overlay_padding(self) -> f32 {
        self.cell_or(space(1))
    }

    fn cell_or(self, window: f32) -> f32 {
        self.cell.unwrap_or(window)
    }
}
