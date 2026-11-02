use crate::style::{ControlSize, Elevation, SPACING_STEP, StyleAtom};

/// The default border width.
pub const BORDER_WIDTH: i16 = 1;
pub(crate) const DISABLED_OPACITY: u16 = 5_000;
pub(crate) const ICON_SIZE: i16 = 20;
pub(crate) const OVERLAY_SHADOW: Elevation = Elevation::Md;

/// Converts a number of spacing steps to pixels.
#[allow(
    clippy::cast_lossless,
    reason = "u8 to f32 is exact; From is not const"
)]
#[must_use]
pub const fn space(steps: u8) -> f32 {
    SPACING_STEP * steps as f32
}

/// Corner radius from the theme: 2, 4, 6, 8, and 9999 px. 0 on a character grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Radius {
    /// The none radius.
    None,
    /// The sm radius.
    Sm,
    /// The base radius.
    Base,
    /// The md radius.
    Md,
    /// The lg radius.
    Lg,
    /// The full radius.
    Full,
}

impl Radius {
    #[must_use]
    pub(crate) const fn atom(self) -> StyleAtom {
        StyleAtom::Radius(self)
    }
}

/// The sizes that a theme gives its controls.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Metrics {
    cell: Option<f32>,
}

impl Metrics {
    /// Returns the current metrics.
    #[must_use]
    pub fn current() -> Self {
        Self {
            cell: crate::style::cell_size(),
        }
    }

    /// Returns the window metrics.
    #[must_use]
    pub const fn window() -> Self {
        Self { cell: None }
    }

    /// Returns whether this metrics is grid.
    #[must_use]
    pub const fn is_grid(self) -> bool {
        self.cell.is_some()
    }

    /// Returns the height of a control of this size.
    #[must_use]
    pub fn control_height(self, size: ControlSize) -> f32 {
        self.cell.unwrap_or(match size {
            ControlSize::Compact => space(5) + 2.0,
            ControlSize::Default => space(7),
            ControlSize::Wide | ControlSize::Grow => space(9),
        })
    }

    /// Returns the width of a control of this size.
    #[must_use]
    pub const fn control_width(self, size: ControlSize) -> f32 {
        match size {
            ControlSize::Compact => space(6),
            ControlSize::Default => space(8),
            ControlSize::Wide | ControlSize::Grow => space(37),
        }
    }

    /// Returns the padding of a control of this size.
    #[must_use]
    pub fn padding(self, size: ControlSize) -> f32 {
        self.cell.unwrap_or(match size {
            ControlSize::Compact => space(1) + 2.0,
            ControlSize::Default => space(2) + 2.0,
            ControlSize::Wide | ControlSize::Grow => space(3) + 2.0,
        })
    }

    /// Returns the gap of a control of this size.
    #[must_use]
    pub fn gap(self, size: ControlSize) -> f32 {
        self.cell.unwrap_or(match size {
            ControlSize::Compact => space(1),
            ControlSize::Default => space(2),
            ControlSize::Wide | ControlSize::Grow => space(3),
        })
    }

    /// Returns the corner radius for `radius`.
    #[must_use]
    pub const fn radius(self, radius: Radius) -> f32 {
        radius_px(radius) as f32
    }

    /// Returns the ring width.
    #[must_use]
    pub const fn ring_width(self) -> f32 {
        2.0
    }

    /// Returns the ring offset.
    #[must_use]
    pub const fn ring_offset(self) -> f32 {
        2.0
    }

    /// Icon size in points, or one cell on the terminal host.
    #[must_use]
    pub fn icon_size(self) -> f32 {
        self.cell.unwrap_or_else(|| f32::from(ICON_SIZE))
    }

    /// Returns the overlay padding.
    #[must_use]
    pub fn overlay_padding(self) -> f32 {
        self.cell.unwrap_or(space(1))
    }
}

pub(crate) const fn radius_px(radius: Radius) -> i16 {
    match radius {
        Radius::None => 0,
        Radius::Sm => 2,
        Radius::Base => 4,
        Radius::Md => 6,
        Radius::Lg => 8,
        Radius::Full => 9_999,
    }
}
