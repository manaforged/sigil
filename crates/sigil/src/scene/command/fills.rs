use crate::hal::Point;
use crate::style::Rgba8;
use smallvec::SmallVec;

#[derive(Clone, Debug, PartialEq)]
pub enum PaintFill {
    Solid(Rgba8),
    LinearGradient(LinearGradient),
    Pattern(PatternKind),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PatternKind {
    Slash {
        color: Rgba8,
        period: f32,
        line_width: f32,
    },
    Checker {
        color_a: Rgba8,
        color_b: Rgba8,
        cell: f32,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct LinearGradient {
    pub start: Point,
    pub end: Point,
    pub stops: SmallVec<[GradientStop; 4]>,
}

impl LinearGradient {
    #[must_use]
    pub fn new(start: Point, end: Point, stops: impl Into<SmallVec<[GradientStop; 4]>>) -> Self {
        Self {
            start,
            end,
            stops: stops.into(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GradientStop {
    pub offset: f32,
    pub color: Rgba8,
}

impl GradientStop {
    #[must_use]
    pub const fn new(offset: f32, color: Rgba8) -> Self {
        Self { offset, color }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum StrokeStyle {
    Solid,
    Dashed(f32),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StrokeAlign {
    Centered,
    Inside,
    Outside,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stroke {
    pub color: Rgba8,
    pub width: f32,
    pub style: StrokeStyle,
    pub align: StrokeAlign,
}
