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
const STEPS: u16 = 1 << 14;

static DECODED: std::sync::LazyLock<[f32; 256]> = std::sync::LazyLock::new(|| {
    let mut table = [0.0; 256];
    for (channel, slot) in (0..=u8::MAX).zip(table.iter_mut()) {
        *slot = decode_unit(f32::from(channel) / 255.0);
    }
    table
});

/// Decodes one sRGB-encoded channel in 0..=1 to linear light.
#[must_use]
pub(crate) fn decode_unit(value: f32) -> f32 {
    if value <= 0.040_45 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

static ENCODED: std::sync::LazyLock<Box<[u8; STEPS as usize + 1]>> =
    std::sync::LazyLock::new(|| {
        let mut table = Box::new([0; STEPS as usize + 1]);
        for (step, byte) in (0..=STEPS).zip(table.iter_mut()) {
            *byte = crate::animation::runtime::linear_channel_to_srgb(
                f32::from(step) / f32::from(STEPS),
            );
        }
        table
    });

#[must_use]
pub(crate) fn decode(channel: u8) -> f32 {
    DECODED.get(usize::from(channel)).copied().unwrap_or(1.0)
}

/// Rounds a channel value in 0.0..=255.0 to its byte, clamping values outside that range.
#[must_use]
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the value is clamped to 0..=255 before the cast"
)]
pub(crate) const fn byte(value: f32) -> u8 {
    value.round().clamp(0.0, 255.0) as u8
}

/// Encodes a linear channel to its sRGB byte through a table.
#[must_use]
pub(crate) fn encode_byte(linear: f32) -> u8 {
    ENCODED.get(table_index(linear)).copied().unwrap_or(u8::MAX)
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the clamped value is in 0..=STEPS + 0.5, so the index fits the table"
)]
fn table_index(linear: f32) -> usize {
    linear.clamp(0.0, 1.0).mul_add(f32::from(STEPS), 0.5) as usize
}

/// The WCAG 2.2 AA contrast ratio for body text.
pub(crate) const AA_TEXT: f32 = 4.5;

/// Returns the WCAG contrast ratio of two opaque colors.
#[must_use]
pub(crate) fn contrast(a: crate::style::Rgba8, b: crate::style::Rgba8) -> f32 {
    let luminance = |color: crate::style::Rgba8| {
        0.0722_f32.mul_add(
            decode(color.b),
            0.7152_f32.mul_add(decode(color.g), 0.2126 * decode(color.r)),
        )
    };
    let (a, b) = (luminance(a), luminance(b));
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}
