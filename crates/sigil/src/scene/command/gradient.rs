use smallvec::SmallVec;

use crate::geometry::Point;
use crate::scene::geometry::MIN_EXTENT;

use super::fills::{GradientStop, LinearGradient, normalize_stops};

kittest_pub! {
    /// The geometry of a gradient, in the same logical pixels as the shape it fills.
    #[doc(hidden)]
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub enum GradientKind {
        /// Colors change along the line from `start` to `end`.
        Linear {
            /// Where offset 0 lies.
            start: Point,
            /// Where offset 1 lies.
            end: Point,
        },
        /// Colors change out from `center` to an ellipse with radii `radius` (x, y).
        Radial {
            /// The center, at offset 0.
            center: Point,
            /// The x and y radii of the ellipse at offset 1.
            radius: [f32; 2],
        },
        /// Colors change clockwise around `center`, starting `angle` degrees clockwise from up.
        Conic {
            /// The point that the colors turn around.
            center: Point,
            /// The start angle in degrees, clockwise from up, as in CSS `conic-gradient(from ..)`.
            angle: f32,
        },
    }
}

kittest_pub! {
    /// A linear, radial, or conic gradient with color stops.
    #[doc(hidden)]
    #[derive(Clone, Debug, PartialEq)]
    pub struct Gradient {
        /// Whether the gradient is linear, radial, or conic, with its points.
        pub kind: GradientKind,
        /// The stops, at offsets from 0 to 1.
        pub stops: SmallVec<[GradientStop; 4]>,
        /// Whether the span from the first to the last stop repeats, as in CSS `repeating-*-gradient`.
        pub repeat: bool,
    }
}

impl Gradient {
    kittest_pub! {
        /// Creates a gradient of `kind` with `stops` that does not repeat.
        #[must_use]
        pub fn new(kind: GradientKind, stops: impl Into<SmallVec<[GradientStop; 4]>>) -> Self {
            Self {
                kind,
                stops: stops.into(),
                repeat: false,
            }
        }
    }

    kittest_pub! {
        /// Sets whether the stop span repeats.
        #[must_use]
        pub const fn repeat(mut self, repeat: bool) -> Self {
            self.repeat = repeat;
            self
        }
    }

    kittest_pub! {
        /// Returns this gradient with stops clamped into offset order in 0..=1 and a degenerate shape reduced to one color.
        #[must_use]
        pub fn normalized(&self) -> Self {
            Self {
                kind: self.kind,
                stops: normalize_stops(&self.stops, self.is_degenerate()),
                repeat: self.repeat,
            }
        }
    }

    kittest_pub! {
        if vector_paint;
        /// The offsets of the first and last stop; `None` when the gradient does not repeat.
        #[must_use]
        pub fn repeat_span(&self) -> Option<[f32; 2]> {
            let (first, last) = (self.stops.first()?.offset, self.stops.last()?.offset);
            (self.repeat && last - first > MIN_EXTENT).then_some([first, last])
        }
    }

    #[cfg(vector_paint)]
    pub(crate) fn spread(&self) -> ([f32; 2], SmallVec<[GradientStop; 4]>) {
        let [first, last] = self.repeat_span().unwrap_or([0.0, 1.0]);
        let width = (last - first).max(f32::EPSILON);
        let stops = self
            .stops
            .iter()
            .map(|stop| {
                GradientStop::new(((stop.offset - first) / width).clamp(0.0, 1.0), stop.color)
            })
            .collect();
        ([first, last], stops)
    }

    pub(crate) fn key(&self, seed: u64) -> u64 {
        let (tag, values) = match self.kind {
            GradientKind::Linear { start, end } => (1, [start.x, start.y, end.x, end.y]),
            GradientKind::Radial { center, radius } => {
                (2, [center.x, center.y, radius[0], radius[1]])
            }
            GradientKind::Conic { center, angle } => (3, [center.x, center.y, angle, 0.0]),
        };
        let hash = values.into_iter().fold(
            crate::fnv::mix(seed, tag + 4 * u64::from(self.repeat)),
            crate::fnv::mix_f32,
        );
        self.stops.iter().fold(hash, |hash, stop| {
            crate::fnv::mix(
                crate::fnv::mix_f32(hash, stop.offset),
                crate::fnv::rgba(stop.color),
            )
        })
    }

    pub(crate) fn shift(&mut self, dx: f32, dy: f32) {
        let moved = |point: &mut Point| {
            point.x += dx;
            point.y += dy;
        };
        match &mut self.kind {
            GradientKind::Linear { start, end } => {
                moved(start);
                moved(end);
            }
            GradientKind::Radial { center, .. } | GradientKind::Conic { center, .. } => {
                moved(center);
            }
        }
    }

    fn is_degenerate(&self) -> bool {
        match self.kind {
            GradientKind::Linear { start, end } => {
                LinearGradient::new(start, end, SmallVec::new()).is_degenerate()
            }
            GradientKind::Radial { radius, .. } => {
                radius.iter().any(|r| r.is_nan() || *r < MIN_EXTENT)
            }
            GradientKind::Conic { angle, .. } => !angle.is_finite(),
        }
    }
}

impl From<LinearGradient> for Gradient {
    fn from(linear: LinearGradient) -> Self {
        Self::new(
            GradientKind::Linear {
                start: linear.start,
                end: linear.end,
            },
            linear.stops,
        )
    }
}
