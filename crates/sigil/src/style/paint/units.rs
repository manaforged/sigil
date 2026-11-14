use crate::style::atom::OPACITY_UNITS;

const LETTER_SPACING_SCALE: f32 = 1_000.0;
const LETTER_SPACING_LIMIT: f32 = 2_000.0;

#[track_caller]
pub(super) fn opacity_units(value: f32) -> u16 {
    debug_assert!(
        value.is_finite() && (0.0..=1.0).contains(&value),
        "opacity must be a finite value in 0.0..=1.0, got {value}",
    );
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "values are bounded to the target range before the cast"
    )]
    let encoded = (value.clamp(0.0, 1.0) * OPACITY_UNITS).round() as u16;
    encoded
}

pub(super) fn letter_spacing_units(em: f32) -> i16 {
    #[allow(
        clippy::cast_possible_truncation,
        reason = "values are bounded to the target range before the cast"
    )]
    let thousandths = (em * LETTER_SPACING_SCALE)
        .round()
        .clamp(-LETTER_SPACING_LIMIT, LETTER_SPACING_LIMIT) as i16;
    thousandths
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the fraction is clamped to 0..=1 before it is encoded in ten-thousandths"
)]
pub(in crate::style) fn fraction_units(value: f32) -> u16 {
    (value.clamp(0.0, 1.0) * 10_000.0).round() as u16
}
