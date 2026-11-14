pub(crate) const OPAQUE: u16 = 10_000;

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the opacity is clamped to 0..=1 before scaling to 0..=10_000"
)]
pub(crate) const fn to_basis(opacity: f32) -> u16 {
    (opacity.clamp(0.0, 1.0) * 10_000.0).round() as u16
}

pub(crate) fn from_basis(basis: u16) -> f32 {
    f32::from(basis) / 10_000.0
}
