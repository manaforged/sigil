use crate::animation::runtime::binding_animate_color::mix;
use crate::geometry::Point;
use crate::scene::geometry::MIN_EXTENT;
use crate::style::Color;
use smallvec::SmallVec;

#[cfg(feature = "kittest")]
/// The fill of a painted area: solid color, gradient, or pattern.
#[derive(Clone, Debug, PartialEq)]
pub enum PaintFill {
    /// The solid paint fill.
    Solid(Color),
    /// The linear gradient paint fill.
    LinearGradient(LinearGradient),
    /// The pattern paint fill.
    Pattern(PatternKind),
}

kittest_pub! {
    /// The shape of a pattern fill.
    #[doc(hidden)]
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub enum PatternKind {
        #[cfg(feature = "kittest")]
        /// The slash pattern kind.
        Slash {
            /// The color.
            color: Color,
            /// The period.
            period: f32,
            /// The line width.
            line_width: f32,
        },
        #[cfg(feature = "kittest")]
        /// The checker pattern kind.
        Checker {
            /// The color a.
            color_a: Color,
            /// The color b.
            color_b: Color,
            /// The cell.
            cell: f32,
        },
        /// The watercolour wash pattern kind.
        Wash(super::wash::Wash),
    }
}

impl PatternKind {
    kittest_pub! {
        if feature = "scene";
        /// Returns this pattern with its values in range.
        #[must_use]
        pub const fn normalized(self) -> Self {
            match self {
                #[cfg(feature = "kittest")]
                Self::Slash {
                    color,
                    period,
                    line_width,
                } => {
                    let period = pattern_length(period);
                    let line_width = if line_width.is_nan() {
                        0.0
                    } else {
                        line_width.clamp(0.0, period)
                    };
                    Self::Slash {
                        color,
                        period,
                        line_width,
                    }
                }
                #[cfg(feature = "kittest")]
                Self::Checker {
                    color_a,
                    color_b,
                    cell,
                } => Self::Checker {
                    color_a,
                    color_b,
                    cell: pattern_length(cell),
                },
                Self::Wash(wash) => Self::Wash(wash.normalized()),
            }
        }
    }

    kittest_pub! {
        if feature = "scene";
        /// The length after which the pattern repeats.
        #[must_use]
        pub const fn tile_extent(self) -> f32 {
            match self.normalized() {
                #[cfg(feature = "kittest")]
                Self::Slash { period, .. } => period,
                #[cfg(feature = "kittest")]
                Self::Checker { cell, .. } => cell * 2.0,
                Self::Wash(_) => WASH_EXTENT,
            }
        }
    }
}

#[cfg(raster)]
const WASH_EXTENT: f32 = 1.0e6;

pub(crate) const TRANSPARENT: Color = Color {
    r: 0,
    g: 0,
    b: 0,
    a: 0,
};

#[cfg(feature = "kittest")]
const MIN_PATTERN_LENGTH: f32 = 1.0;
#[cfg(feature = "kittest")]
const MAX_PATTERN_LENGTH: f32 = 1024.0;

#[cfg(feature = "kittest")]
const fn pattern_length(length: f32) -> f32 {
    if length.is_nan() {
        MIN_PATTERN_LENGTH
    } else {
        length.clamp(MIN_PATTERN_LENGTH, MAX_PATTERN_LENGTH)
    }
}

kittest_pub! {
    /// A gradient along a straight axis.
    #[doc(hidden)]
    #[derive(Clone, Debug, PartialEq)]
    pub struct LinearGradient {
        /// The start.
        pub start: Point,
        /// The end.
        pub end: Point,
        /// The stops.
        pub stops: SmallVec<[GradientStop; 4]>,
    }
}

impl LinearGradient {
    kittest_pub! {
        /// Creates a gradient from `start` to `end` with `stops`.
        #[must_use]
        pub fn new(
            start: Point,
            end: Point,
            stops: impl Into<SmallVec<[GradientStop; 4]>>,
        ) -> Self {
            Self {
                start,
                end,
                stops: stops.into(),
            }
        }
    }

    pub(super) fn is_degenerate(&self) -> bool {
        let length = (self.end.x - self.start.x).hypot(self.end.y - self.start.y);
        length.is_nan() || length < MIN_EXTENT
    }
}

pub(super) fn normalize_stops(
    stops: &[GradientStop],
    degenerate: bool,
) -> SmallVec<[GradientStop; 4]> {
    let mut floor = f32::NEG_INFINITY;
    let placed: SmallVec<[GradientStop; 4]> = stops
        .iter()
        .filter(|stop| stop.offset.is_finite())
        .map(|stop| {
            floor = floor.max(stop.offset);
            GradientStop::new(floor, stop.color)
        })
        .collect();
    match placed.last() {
        None => smallvec::smallvec![GradientStop::new(0.0, TRANSPARENT)],
        Some(last) if degenerate => smallvec::smallvec![GradientStop::new(0.0, last.color)],
        Some(_) => clamp_stops(&placed),
    }
}

fn clamp_stops(sorted: &[GradientStop]) -> SmallVec<[GradientStop; 4]> {
    let mut out: SmallVec<[GradientStop; 4]> = SmallVec::new();
    if sorted.first().is_some_and(|stop| stop.offset < 0.0) {
        out.push(GradientStop::new(0.0, color_at(sorted, 0.0)));
    }
    out.extend(
        sorted
            .iter()
            .copied()
            .filter(|stop| (0.0..=1.0).contains(&stop.offset)),
    );
    if sorted.last().is_some_and(|stop| stop.offset > 1.0) {
        out.push(GradientStop::new(1.0, color_at(sorted, 1.0)));
    }
    out
}

/// The color at offset `t` of stops in offset order, mixed in premultiplied gamma-encoded sRGB.
pub(crate) fn color_at(sorted: &[GradientStop], t: f32) -> Color {
    let span = sorted.windows(2).find_map(|pair| match pair {
        [a, b] if a.offset <= t && t < b.offset => Some((*a, *b)),
        _ => None,
    });
    match (span, sorted.first(), sorted.last()) {
        (Some((a, b)), _, _) => mix(a.color, b.color, (t - a.offset) / (b.offset - a.offset)),
        (None, Some(first), _) if t < first.offset => first.color,
        (None, _, Some(last)) => last.color,
        (None, _, None) => TRANSPARENT,
    }
}

kittest_pub! {
    /// A color at one offset of a gradient.
    #[doc(hidden)]
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub struct GradientStop {
        /// The offset.
        pub offset: f32,
        /// The color.
        pub color: Color,
    }
}

impl GradientStop {
    kittest_pub! {
        /// Creates a stop with `color` at `offset`.
        #[must_use]
        pub const fn new(offset: f32, color: Color) -> Self {
            Self { offset, color }
        }
    }
}

kittest_pub! {
    /// The dash style of a stroke.
    #[doc(hidden)]
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub enum StrokeStyle {
        /// The solid stroke style.
        Solid,
        #[cfg(feature = "kittest")]
        /// The dashed stroke style.
        Dashed(f32),
    }
}

kittest_pub! {
    /// Where a stroke sits relative to a shape's edge.
    #[doc(hidden)]
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub enum StrokeAlign {
        #[cfg(feature = "kittest")]
        /// The centered stroke align.
        Centered,
        /// The inside stroke align.
        Inside,
        /// The outside stroke align.
        Outside,
    }
}

kittest_pub! {
    /// The width, color, and style of an outline.
    #[doc(hidden)]
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub struct Stroke {
        /// The color.
        pub color: Color,
        /// The width.
        pub width: f32,
        /// The style.
        pub style: StrokeStyle,
        /// The align.
        pub align: StrokeAlign,
    }
}

/// The shape at the open ends of a stroked path.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum StrokeCap {
    /// Ends flat at the end point.
    #[default]
    Butt,
    /// Ends in a half circle around the end point.
    Round,
    /// Ends in a half square past the end point.
    Square,
}

/// The shape where two stroked segments meet.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum StrokeJoin {
    /// Extends the outer edges to a point.
    #[default]
    Miter,
    /// Rounds the corner with a circle.
    Round,
    /// Cuts the corner straight across.
    Bevel,
}

/// Which areas of a self-crossing path count as inside.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum FillRule {
    /// Inside where the winding number is not zero.
    #[default]
    NonZero,
    /// Inside where the winding number is odd.
    EvenOdd,
}

/// The paint of a path's fill or stroke, in path units.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum PathPaint {
    /// One color.
    Solid(Color),
    /// A linear, radial, or conic gradient.
    Gradient(super::gradient::Gradient),
}
