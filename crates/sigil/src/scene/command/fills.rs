use crate::geometry::Point;
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

impl PatternKind {
    #[must_use]
    pub const fn normalized(self) -> Self {
        match self {
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
            Self::Checker {
                color_a,
                color_b,
                cell,
            } => Self::Checker {
                color_a,
                color_b,
                cell: pattern_length(cell),
            },
        }
    }

    #[must_use]
    pub const fn tile_extent(self) -> f32 {
        match self.normalized() {
            Self::Slash { period, .. } => period,
            Self::Checker { cell, .. } => cell * 2.0,
        }
    }
}

pub(crate) const TRANSPARENT: Rgba8 = Rgba8 {
    r: 0,
    g: 0,
    b: 0,
    a: 0,
};

const MIN_PATTERN_LENGTH: f32 = 1.0;
const MAX_PATTERN_LENGTH: f32 = 1024.0;

const fn pattern_length(length: f32) -> f32 {
    if length.is_nan() {
        MIN_PATTERN_LENGTH
    } else {
        length.clamp(MIN_PATTERN_LENGTH, MAX_PATTERN_LENGTH)
    }
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

    #[must_use]
    pub fn normalized(&self) -> Self {
        let mut sorted: SmallVec<[GradientStop; 4]> = self
            .stops
            .iter()
            .copied()
            .filter(|stop| stop.offset.is_finite())
            .collect();
        sorted.sort_by(|a, b| a.offset.total_cmp(&b.offset));
        let stops = match sorted.last() {
            None => smallvec::smallvec![GradientStop::new(0.0, TRANSPARENT)],
            Some(last) if self.is_degenerate() => {
                smallvec::smallvec![GradientStop::new(0.0, last.color)]
            }
            Some(_) => clamp_stops(&sorted),
        };
        Self::new(self.start, self.end, stops)
    }

    fn is_degenerate(&self) -> bool {
        let length = (self.end.x - self.start.x).hypot(self.end.y - self.start.y);
        length.is_nan() || length < DEGENERATE_LENGTH
    }
}

const DEGENERATE_LENGTH: f32 = 1.0 / 4096.0;

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

fn color_at(sorted: &[GradientStop], t: f32) -> Rgba8 {
    let span = sorted.windows(2).find_map(|pair| match pair {
        [a, b] if a.offset <= t && t < b.offset => Some((*a, *b)),
        _ => None,
    });
    match (span, sorted.first(), sorted.last()) {
        (Some((a, b)), _, _) => {
            lerp_color(a.color, b.color, (t - a.offset) / (b.offset - a.offset))
        }
        (None, Some(first), _) if t < first.offset => first.color,
        (None, _, Some(last)) => last.color,
        (None, _, None) => TRANSPARENT,
    }
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the channel is rounded and clamped to 0..=255 before the cast"
)]
fn lerp_color(a: Rgba8, b: Rgba8, amount: f32) -> Rgba8 {
    let channel = |x: u8, y: u8| {
        let value = (f32::from(y) - f32::from(x)).mul_add(amount, f32::from(x));
        value.round().clamp(0.0, 255.0) as u8
    };
    Rgba8 {
        r: channel(a.r, b.r),
        g: channel(a.g, b.g),
        b: channel(a.b, b.b),
        a: channel(a.a, b.a),
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
