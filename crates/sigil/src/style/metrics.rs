use crate::style::{ControlSize, Elevation, SPACING_STEP, StyleAtom, Token};

/// The default border width.
pub const BORDER_WIDTH: i16 = 1;
pub(crate) const DISABLED_OPACITY: u16 = 5_000;
pub(crate) static DIMMED: &[StyleAtom] = &[StyleAtom::Opacity(DISABLED_OPACITY)];
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
#[non_exhaustive]
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
    /// The metrics of the active cell grid, or the window metrics when no cell size is set.
    #[must_use]
    pub fn current() -> Self {
        Self {
            cell: crate::style::cell_size(),
        }
    }

    /// The metrics of a window, with no cell grid.
    #[must_use]
    pub const fn window() -> Self {
        Self { cell: None }
    }

    /// Returns whether this metrics is grid.
    #[must_use]
    pub const fn is_grid(self) -> bool {
        self.cell.is_some()
    }

    /// The height of a control of `size` in pixels; one cell on a cell grid.
    #[must_use]
    pub fn control_height(self, size: ControlSize) -> f32 {
        self.cell.unwrap_or(match size {
            ControlSize::Compact => space(6),
            ControlSize::Default => space(7),
            ControlSize::Wide => space(9),
        })
    }

    /// The width of a control of `size` in pixels.
    #[must_use]
    pub const fn control_width(self, size: ControlSize) -> f32 {
        match size {
            ControlSize::Compact => space(6),
            ControlSize::Default => space(8),
            ControlSize::Wide => space(37),
        }
    }

    /// The padding of a control of `size` in pixels; one cell on a cell grid.
    #[must_use]
    pub fn control_padding(self, size: ControlSize) -> f32 {
        self.cell.unwrap_or(match size {
            ControlSize::Compact => space(1) + 2.0,
            ControlSize::Default => space(2) + 2.0,
            ControlSize::Wide => space(3) + 2.0,
        })
    }

    /// The gap of a control of `size` in pixels; one cell on a cell grid.
    #[must_use]
    pub fn control_gap(self, size: ControlSize) -> f32 {
        self.cell.unwrap_or(match size {
            ControlSize::Compact => space(1),
            ControlSize::Default => space(2),
            ControlSize::Wide => space(3),
        })
    }

    /// Returns the corner radius for `radius`.
    #[must_use]
    pub const fn radius(self, radius: Radius) -> f32 {
        radius_px(radius) as f32
    }

    /// The stroke width of the focus ring: 2 px.
    #[must_use]
    pub const fn ring_width(self) -> f32 {
        2.0
    }

    /// The distance between a node and its focus ring: 2 px.
    #[must_use]
    pub const fn ring_offset(self) -> f32 {
        2.0
    }

    /// Icon size in points, or one cell on the terminal host.
    #[must_use]
    pub fn icon_size(self) -> f32 {
        self.cell.unwrap_or_else(|| f32::from(ICON_SIZE))
    }

    /// The padding of an overlay in pixels; one cell on a cell grid.
    #[must_use]
    pub fn overlay_padding(self) -> f32 {
        self.cell.unwrap_or(space(1))
    }
}

/// A place in the visual stack: its fill, border, corners, and shadow, set together.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Layer {
    /// The window background.
    Page,
    /// A recessed well, such as a code block or a sidebar.
    Sunken,
    /// A card or panel above the page.
    Raised,
    /// A menu, popup, or dialog above everything else.
    Overlay,
}

impl Layer {
    #[must_use]
    pub(crate) const fn atoms(self) -> &'static [StyleAtom] {
        match self {
            Self::Page => const { &[Token::Surface.background()] },
            Self::Sunken => const { &[Token::SurfaceSubtle.background(), Radius::Md.atom()] },
            Self::Raised => {
                const {
                    &[
                        Token::Surface.background(),
                        StyleAtom::BorderWidth(BORDER_WIDTH),
                        Token::Border.border(),
                        StyleAtom::Shadow(Elevation::Sm),
                        Radius::Md.atom(),
                    ]
                }
            }
            Self::Overlay => {
                const {
                    &[
                        Token::Overlay.background(),
                        StyleAtom::BorderWidth(BORDER_WIDTH),
                        Token::OverlayBorder.border(),
                        StyleAtom::Shadow(OVERLAY_SHADOW),
                        Radius::Md.atom(),
                    ]
                }
            }
        }
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
