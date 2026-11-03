#[must_use]
pub(crate) fn encode(linear: f32) -> f32 {
    let v = linear.clamp(0.0, 1.0);
    if v <= 0.003_130_8 {
        v * 12.92
    } else {
        1.055_f32.mul_add(v.powf(1.0 / 2.4), -0.055)
    }
}

/// Steps in the linear-to-byte table; fine enough that a lookup matches the exact rounding.
#[cfg(feature = "scene")]
const STEPS: usize = 1 << 14;

static DECODED: std::sync::LazyLock<[f32; 256]> = std::sync::LazyLock::new(|| {
    std::array::from_fn(|channel| {
        let v = channel as f32 / 255.0;
        if v <= 0.040_45 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    })
});

#[cfg(feature = "scene")]
static ENCODED: std::sync::LazyLock<Box<[u8; STEPS + 1]>> = std::sync::LazyLock::new(|| {
    let mut table = Box::new([0; STEPS + 1]);
    for (step, byte) in table.iter_mut().enumerate() {
        *byte = (encode(step as f32 / STEPS as f32) * 255.0).round() as u8;
    }
    table
});

#[must_use]
pub(crate) fn decode(channel: u8) -> f32 {
    DECODED[usize::from(channel)]
}

/// Encodes a linear channel to its sRGB byte through a table.
#[cfg(feature = "scene")]
#[must_use]
pub(crate) fn encode_byte(linear: f32) -> u8 {
    ENCODED[(linear.clamp(0.0, 1.0) * STEPS as f32 + 0.5) as usize]
}
