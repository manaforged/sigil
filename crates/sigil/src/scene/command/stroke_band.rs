use crate::geometry::{Point, Rect};

use super::fills::{Stroke, StrokeAlign, StrokeStyle};

const MIN_DASH_GAP: f32 = 1.0 / 16.0;
const MIN_DASH_LENGTH: f32 = 1.0;
const DASH_PER_WIDTH: f32 = 3.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct StrokeDash {
    pub(crate) length: f32,
    pub(crate) gap: f32,
}

impl StrokeDash {
    pub(crate) fn covers(self, local: Point, center: [f64; 2]) -> bool {
        let [cx, cy] = center;
        let arc = (f64::from(local.x) - cx).abs() + (f64::from(local.y) - cy).abs();
        arc.rem_euclid(self.period()) < f64::from(self.length)
    }

    #[allow(
        clippy::cast_possible_truncation,
        reason = "the folded offset stays within one period of the reach"
    )]
    pub(crate) fn fold(self, offset: f64, reach: f32) -> f32 {
        let reach = f64::from(reach);
        if !offset.is_finite() {
            return 0.0;
        }
        if offset.abs() <= reach {
            return offset as f32;
        }
        let period = self.period();
        let rest = offset.abs().rem_euclid(period);
        let turns = ((reach - rest) / period).ceil().max(0.0);
        (offset.signum() * turns.mul_add(period, rest)) as f32
    }

    fn period(self) -> f64 {
        f64::from(self.length) + f64::from(self.gap)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct StrokeBand {
    pub(crate) inside: f32,
    pub(crate) outside: f32,
    pub(crate) dash: Option<StrokeDash>,
}

impl StrokeBand {
    pub(crate) fn of(stroke: Stroke, rect: Rect) -> Option<Self> {
        let width = stroke.width;
        if !width.is_finite() || width <= 0.0 {
            return None;
        }
        let (inside, outside) = match stroke.align {
            StrokeAlign::Centered => (width / 2.0, width / 2.0),
            StrokeAlign::Inside => (width, 0.0),
            StrokeAlign::Outside => (0.0, width),
        };
        let depth = rect.width.min(rect.height).max(0.0) / 2.0;
        Some(Self {
            inside: inside.min(depth),
            outside,
            dash: dash(width, stroke.style),
        })
    }

    pub(crate) fn reach(stroke: Option<Stroke>, rect: Rect) -> f32 {
        stroke
            .and_then(|stroke| Self::of(stroke, rect))
            .map_or(0.0, |band| band.outside)
    }

    pub(crate) fn ink(stroke: Option<Stroke>, rect: Rect) -> Rect {
        let reach = Self::reach(stroke, rect);
        Rect::new(
            Point::new(rect.origin.x - reach, rect.origin.y - reach),
            reach.mul_add(2.0, rect.width),
            reach.mul_add(2.0, rect.height),
        )
    }
}

fn dash(width: f32, style: StrokeStyle) -> Option<StrokeDash> {
    let StrokeStyle::Dashed(gap) = style else {
        return None;
    };
    if !gap.is_finite() || gap < MIN_DASH_GAP {
        return None;
    }
    let length = (width * DASH_PER_WIDTH).max(MIN_DASH_LENGTH);
    (length + gap)
        .is_finite()
        .then_some(StrokeDash { length, gap })
}
