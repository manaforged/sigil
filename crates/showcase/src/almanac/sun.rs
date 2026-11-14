use std::f32::consts::{PI, TAU};

const AXIAL_TILT_DEGREES: f32 = 23.44;
const DAYS_PER_YEAR: f32 = 365.0;
const SOLSTICE_OFFSET_DAYS: f32 = 10.0;
const HOURS_PER_DAY: f32 = 24.0;
const NOON: f32 = 12.0;
const DEGREES_PER_HOUR: f32 = 15.0;

/// The sun over one latitude on one day of the year, in local solar time.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Sun {
    latitude: f32,
    declination: f32,
}

impl Sun {
    /// The sun at `latitude` degrees north on day `day` of the year.
    #[must_use]
    pub(crate) fn new(latitude: f32, day: f32) -> Self {
        let season = TAU * (day + SOLSTICE_OFFSET_DAYS) / DAYS_PER_YEAR;
        Self {
            latitude: latitude.to_radians(),
            declination: -AXIAL_TILT_DEGREES.to_radians() * season.cos(),
        }
    }

    /// Degrees above the horizon at `hour`; negative after sunset.
    #[must_use]
    pub(crate) fn altitude(self, hour: f32) -> f32 {
        let angle = ((hour - NOON) * DEGREES_PER_HOUR).to_radians();
        let height = self.latitude.sin().mul_add(
            self.declination.sin(),
            self.latitude.cos() * self.declination.cos() * angle.cos(),
        );
        height.clamp(-1.0, 1.0).asin().to_degrees()
    }

    /// Hours between sunrise and sunset.
    #[must_use]
    pub(crate) fn daylight(self) -> f32 {
        let edge = -self.latitude.tan() * self.declination.tan();
        edge.clamp(-1.0, 1.0).acos() / PI * HOURS_PER_DAY
    }

    /// The hour the sun crosses the horizon going up.
    #[must_use]
    pub(crate) fn sunrise(self) -> f32 {
        NOON - self.daylight() / 2.0
    }

    /// The hour the sun crosses the horizon going down.
    #[must_use]
    pub(crate) fn sunset(self) -> f32 {
        NOON + self.daylight() / 2.0
    }
}

/// Formats `hours` as `HH:MM`.
#[must_use]
pub(crate) fn clock(hours: f32) -> String {
    let (whole, minutes) = split(hours.rem_euclid(HOURS_PER_DAY));
    format!("{whole:02}:{minutes:02}")
}

/// Formats `hours` as a length of time, such as `10 h 44 min`.
#[must_use]
pub(crate) fn span(hours: f32) -> String {
    let (whole, minutes) = split(hours);
    format!("{whole} h {minutes:02} min")
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "minutes in a day are a small non-negative whole number"
)]
fn split(hours: f32) -> (u32, u32) {
    let minutes = (hours.max(0.0) * 60.0).round() as u32;
    (minutes / 60, minutes % 60)
}
