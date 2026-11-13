use crate::geometry::Rect;

use super::effects::DecorationKind;
use super::payloads::DrawDecoration;

#[cfg(terminal_host)]
const WAVE_STROKE: f32 = 0.8;
#[cfg(terminal_host)]
const WAVE_AMPLITUDE: f32 = 0.75;
const WAVE_MIN_PERIOD: f32 = 1.0;
#[cfg(terminal_host)]
const WAVE_PERIOD_PER_REACH: f32 = 4.0;

/// The position and size of a text decoration.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DecorationGeometry {
    /// The rectangle the decoration is drawn in.
    pub(crate) band: Rect,
    /// The y coordinate of the middle of the line, halfway down `band`.
    pub(crate) center: f32,
    /// The thickness of the line, at most the height of `band`.
    pub(crate) thickness: f32,
    /// How far a wavy line rises and falls from `center`; 0 for a straight line.
    pub(crate) amplitude: f32,
    /// The horizontal length of one full wave, at least 1.
    pub(crate) period: f32,
}

impl DecorationGeometry {
    #[cfg(terminal_host)]
    /// Returns the height of a wavy line at `x`.
    #[must_use]
    pub(crate) fn wave_y(&self, x: f32) -> f32 {
        self.amplitude
            .mul_add((x / self.period * std::f32::consts::TAU).sin(), self.center)
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

    #[cfg(terminal_host)]
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
            #[cfg(terminal_host)]
            DecorationKind::WavyUnderline => Some(wave(straight, width, self.period)),
        }
    }
}

#[cfg(terminal_host)]
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
