use crate::geometry::Rect;

use super::effects::DecorationKind;
use super::payloads::DrawDecoration;

#[cfg(any(test, feature = "kittest", feature = "terminal"))]
const WAVE_STROKE: f32 = 0.8;
#[cfg(any(test, feature = "kittest", feature = "terminal"))]
const WAVE_AMPLITUDE: f32 = 0.75;
const WAVE_MIN_PERIOD: f32 = 1.0;
#[cfg(any(test, feature = "kittest", feature = "terminal"))]
const WAVE_PERIOD_PER_REACH: f32 = 4.0;
#[cfg(any(test, feature = "kittest", feature = "terminal"))]
const TAU: f32 = std::f32::consts::TAU;

/// The position and size of a text decoration.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DecorationGeometry {
    /// The band.
    pub(crate) band: Rect,
    /// The center.
    pub(crate) center: f32,
    /// The thickness.
    pub(crate) thickness: f32,
    /// The amplitude.
    pub(crate) amplitude: f32,
    /// The period.
    pub(crate) period: f32,
}

impl DecorationGeometry {
    #[cfg(any(test, feature = "kittest", feature = "terminal"))]
    /// Returns the height of a wavy line at `x`.
    #[must_use]
    pub(crate) fn wave_y(&self, x: f32) -> f32 {
        self.amplitude
            .mul_add((x / self.period * TAU).sin(), self.center)
    }

    /// Returns the line.
    #[must_use]
    pub(crate) fn line(&self) -> Rect {
        Rect::new(
            crate::geometry::Point::new(self.band.origin.x, self.center - self.thickness / 2.0),
            self.band.width,
            self.thickness,
        )
    }

    #[cfg(any(test, feature = "kittest", feature = "terminal"))]
    /// Returns the reach.
    #[must_use]
    pub(crate) fn reach(&self) -> f32 {
        self.amplitude + self.thickness / 2.0
    }
}

impl DrawDecoration {
    /// Returns the geometry.
    #[must_use]
    pub(crate) fn geometry(&self) -> Option<DecorationGeometry> {
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
            #[cfg(any(test, feature = "kittest", feature = "terminal"))]
            DecorationKind::WavyUnderline => Some(wave(straight, width, self.period)),
        }
    }
}

#[cfg(any(test, feature = "kittest", feature = "terminal"))]
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
