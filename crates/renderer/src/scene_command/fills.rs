//! Fill and stroke model types (gradients, patterns, stroke style).

use sigil_hal::Point;
use sigil_style::Rgba8;
use smallvec::SmallVec;

/// Rectangle fill model shared by future style adapters.
#[derive(Clone, Debug, PartialEq)]
pub enum Fill {
    /// Solid sRGB colour.
    Solid(Rgba8),
    /// Linear gradient over scene logical pixels.
    LinearGradient(LinearGradient),
    /// Tiled pattern fill.
    Pattern(PatternKind),
}

/// Tiled pattern fill variant.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PatternKind {
    /// Diagonal (45°) stripe pattern.
    ///
    /// `color` is the stripe colour in sRGB. `period` and `line_width` are
    /// in logical pixels relative to the rect centre — the same coordinate
    /// frame used by the gradient pipeline.
    Slash {
        /// Stripe colour in sRGB.
        color: Rgba8,
        /// Stripe period in logical pixels.
        period: f32,
        /// Stripe line width in logical pixels.
        line_width: f32,
    },
    /// Alternating square checker pattern.
    ///
    /// Cell boundaries are in logical pixels relative to the rect centre.
    Checker {
        /// First cell colour in sRGB.
        color_a: Rgba8,
        /// Second cell colour in sRGB.
        color_b: Rgba8,
        /// Cell side length in logical pixels.
        cell: f32,
    },
}

/// Linear gradient with stops authored as sRGB colours.
///
/// Stop colours are converted to linear premultiplied GPU storage at
/// upload time so interpolation happens in linear space.
#[derive(Clone, Debug, PartialEq)]
pub struct LinearGradient {
    /// Gradient start point in scene logical pixels.
    pub start: Point,
    /// Gradient end point in scene logical pixels.
    pub end: Point,
    /// Ordered gradient stops. The renderer consumes at most 8 stops.
    pub stops: SmallVec<[GradientStop; 4]>,
}

impl LinearGradient {
    /// Create a linear gradient from ordered stops.
    #[must_use]
    pub fn new(start: Point, end: Point, stops: impl Into<SmallVec<[GradientStop; 4]>>) -> Self {
        Self {
            start,
            end,
            stops: stops.into(),
        }
    }
}

/// One colour stop in a linear gradient.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GradientStop {
    /// Stop offset in `[0, 1]`.
    pub offset: f32,
    /// Stop colour in sRGB.
    pub color: Rgba8,
}

impl GradientStop {
    /// Create a gradient stop.
    #[must_use]
    pub const fn new(offset: f32, color: Rgba8) -> Self {
        Self { offset, color }
    }
}

/// Rectangle stroke dash model.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum StrokeStyle {
    /// Continuous stroke.
    Solid,
    /// Dashed stroke, carrying the screen-space gap length in pixels.
    Dashed(f32),
}

/// Stroke alignment relative to the rectangle SDF boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StrokeAlign {
    /// Stroke is centered on the rectangle edge.
    Centered,
    /// Stroke is shifted toward the rectangle interior.
    Inside,
    /// Stroke is shifted toward the rectangle exterior.
    Outside,
}

/// Solid-colour rectangle stroke.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stroke {
    /// Straight-alpha sRGB stroke colour. Resolved to the render
    /// target's blend space at instance build, the same as rect fills
    /// (pre-DR-057 this was a raw `[f32; 4]` passed to the shader
    /// unconverted, which made stroke colours format-dependent).
    pub color: Rgba8,
    /// Stroke width in logical pixels.
    pub width: f32,
    /// Solid or dashed stroke model.
    pub style: StrokeStyle,
    /// Stroke alignment.
    pub align: StrokeAlign,
}
