#[must_use]
pub(crate) fn encode(linear: f32) -> f32 {
    let v = linear.clamp(0.0, 1.0);
    if v <= 0.003_130_8 {
        v * 12.92
    } else {
        1.055_f32.mul_add(v.powf(1.0 / 2.4), -0.055)
    }
}

#[must_use]
pub(crate) fn decode(channel: u8) -> f32 {
    let v = f32::from(channel) / 255.0;
    if v <= 0.040_45 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}
