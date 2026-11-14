use crate::style::{ControlSize, Elevation, SPACING_STEP, StyleAtom, Token};

/// The default border width: 1 px.
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
    /// Square corners: 0 px.
    None,
    /// A 2 px corner radius.
    Sm,
    /// A 4 px corner radius.
    Base,
    /// A 6 px corner radius.
    Md,
    /// An 8 px corner radius.
    Lg,
    /// A 9999 px corner radius, which rounds the box into a pill or a circle.
    Full,
}

impl Radius {
    #[must_use]
    pub(crate) const fn atom(self) -> StyleAtom {
        StyleAtom::Radius(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Heights {
    compact: f32,
    default: f32,
    wide: f32,
}

impl Heights {
    const DEFAULT: Self = Self {
        compact: space(6),
        default: space(7),
        wide: space(9),
    };

    const fn slot(&mut self, size: ControlSize) -> &mut f32 {
        match size {
            ControlSize::Compact => &mut self.compact,
            ControlSize::Default => &mut self.default,
            ControlSize::Wide => &mut self.wide,
        }
    }

    const fn get(mut self, size: ControlSize) -> f32 {
        *self.slot(size)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Radii {
    none: f32,
    sm: f32,
    base: f32,
    md: f32,
    lg: f32,
    full: f32,
}

impl Radii {
    const DEFAULT: Self = Self {
        none: 0.0,
        sm: 2.0,
        base: 4.0,
        md: 6.0,
        lg: 8.0,
        full: 9_999.0,
    };

    const fn slot(&mut self, radius: Radius) -> &mut f32 {
        match radius {
            Radius::None => &mut self.none,
            Radius::Sm => &mut self.sm,
            Radius::Base => &mut self.base,
            Radius::Md => &mut self.md,
            Radius::Lg => &mut self.lg,
            Radius::Full => &mut self.full,
        }
    }

    const fn get(mut self, radius: Radius) -> f32 {
        *self.slot(radius)
    }
}

const fn clamp_non_neg(px: f32) -> f32 {
    if px.is_finite() && px >= 0.0 { px } else { 0.0 }
}

/// The sizes that a theme gives its controls.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Metrics {
    cell: Option<f32>,
    heights: Heights,
    radii: Radii,
}

impl Metrics {
    /// The metrics of the active cell grid, or the window metrics when no cell size is set.
    #[must_use]
    pub fn current() -> Self {
        crate::style::theme::ambient::current_metrics()
    }

    /// The metrics of a window, with no cell grid.
    #[must_use]
    pub const fn window() -> Self {
        Self {
            cell: None,
            heights: Heights::DEFAULT,
            radii: Radii::DEFAULT,
        }
    }

    pub(crate) const fn with_cell(mut self, cell: Option<f32>) -> Self {
        self.cell = cell;
        self
    }

    pub(crate) const fn with_control_height(mut self, size: ControlSize, px: f32) -> Self {
        *self.heights.slot(size) = clamp_non_neg(px);
        self
    }

    pub(crate) const fn with_radius(mut self, radius: Radius, px: f32) -> Self {
        *self.radii.slot(radius) = clamp_non_neg(px);
        self
    }

    /// Returns whether this metrics is grid.
    #[must_use]
    pub const fn is_grid(self) -> bool {
        self.cell.is_some()
    }

    /// The height of a control of `size` in pixels; one cell on a cell grid.
    #[must_use]
    pub fn control_height(self, size: ControlSize) -> f32 {
        self.cell.unwrap_or_else(|| self.heights.get(size))
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
        self.radii.get(radius)
    }

    #[allow(
        clippy::cast_possible_truncation,
        reason = "value is rounded and clamped to 0..=i16::MAX before the cast"
    )]
    pub(crate) fn radius_i16(self, radius: Radius) -> i16 {
        self.radius(radius).round().clamp(0.0, f32::from(i16::MAX)) as i16
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

    /// Snaps a track or scrollbar size in pixels to whole terminal cells when on a cell grid.
    #[must_use]
    pub fn track_size(self, px: f32) -> f32 {
        self.cell
            .filter(|c| *c > 0.0)
            .map_or(px, |cell| (px / cell).round().max(1.0) * cell)
    }
}

/// A place in the visual stack: its fill, border, corners, and shadow, set together.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Layer {
    /// The window background, under every other layer.
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
                        StyleAtom::Shadow(crate::style::atom::ShadowAtom::elevation(Elevation::Sm)),
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
                        StyleAtom::Shadow(crate::style::atom::ShadowAtom::elevation(OVERLAY_SHADOW)),
                        Radius::Md.atom(),
                    ]
                }
            }
        }
    }
}
