pub(super) fn polarity_flag(color: [f32; 4], format: wgpu::TextureFormat) -> f32 {
    let lum = 0.0722f32.mul_add(color[2], 0.7152f32.mul_add(color[1], 0.2126 * color[0]));

    let perceptual = if format.is_srgb() {
        linear_to_srgb(lum)
    } else {
        lum
    };

    f32::from(u8::from(perceptual >= 0.5))
}

pub(super) fn linear_to_srgb(value: f32) -> f32 {
    let v = value.clamp(0.0, 1.0);

    if v <= 0.003_130_8 {
        v * 12.92
    } else {
        1.055f32.mul_add(v.powf(1.0 / 2.4), -0.055)
    }
}
