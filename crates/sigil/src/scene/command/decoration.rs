use crate::geometry::{Point, Rect};

use super::effects::DecorationKind;
use super::payloads::DrawDecoration;

const WAVE_STROKE: f32 = 0.8;
const WAVE_AMPLITUDE: f32 = 0.75;
const WAVE_MIN_PERIOD: f32 = 1.0;
const WAVE_PERIOD_PER_REACH: f32 = 4.0;
const TAU: f32 = std::f32::consts::TAU;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DecorationGeometry {
    pub band: Rect,
    pub center: f32,
    pub thickness: f32,
    pub amplitude: f32,
    pub period: f32,
}

impl DecorationGeometry {
    #[must_use]
    pub fn wave_y(&self, x: f32) -> f32 {
        self.amplitude
            .mul_add((x / self.period * TAU).sin(), self.center)
    }

    #[must_use]
    pub fn line(&self) -> Rect {
        Rect::new(
            Point::new(self.band.origin.x, self.center - self.thickness / 2.0),
            self.band.width,
            self.thickness,
        )
    }

    #[must_use]
    pub fn reach(&self) -> f32 {
        self.amplitude + self.thickness / 2.0
    }
}

impl DrawDecoration {
    #[must_use]
    pub fn geometry(&self) -> Option<DecorationGeometry> {
        let band = self.band;
        let finite = [band.origin.x, band.origin.y, band.width, band.height]
            .iter()
            .all(|v| v.is_finite());
        let drawable = band.width > 0.0 && band.height > 0.0;
        let width = self.line_width;
        if !finite || !drawable || !width.is_finite() || width <= 0.0 {
            return None;
        }
        let straight = DecorationGeometry {
            band,
            center: band.origin.y + band.height / 2.0,
            thickness: width.min(band.height),
            amplitude: 0.0,
            period: WAVE_MIN_PERIOD,
        };
        match self.kind {
            DecorationKind::Underline | DecorationKind::Strikethrough => Some(straight),
            DecorationKind::WavyUnderline => Some(wave(straight, width, self.period)),
        }
    }
}

fn wave(straight: DecorationGeometry, width: f32, period: f32) -> DecorationGeometry {
    let height = straight.band.height;
    let thickness = (width * WAVE_STROKE).min(height / 2.0);
    let amplitude = (width * WAVE_AMPLITUDE).min((height - thickness) / 2.0);
    let floor = (WAVE_PERIOD_PER_REACH * (amplitude + thickness)).max(WAVE_MIN_PERIOD);
    let period = if period.is_finite() {
        period.max(floor)
    } else {
        floor
    };
    DecorationGeometry {
        thickness,
        amplitude,
        period,
        ..straight
    }
}
