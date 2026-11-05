use core::sync::atomic::{AtomicU32, Ordering};

use crate::style::StyleAtom;

static SCALE: AtomicU32 = AtomicU32::new(0x3F80_0000);

/// Scales the named text sizes (`text-xs` to `text-xl`); 1.0 keeps them as they are.
pub fn text_scale(scale: f32) {
    let scale = if scale > 0.0 { scale } else { 1.0 };
    SCALE.store(scale.to_bits(), Ordering::Relaxed);
}

pub(crate) fn current() -> u32 {
    SCALE.load(Ordering::Relaxed)
}

#[allow(
    clippy::cast_possible_truncation,
    reason = "named sizes are small, and the result is clamped to i16"
)]
pub(crate) fn scaled(atom: StyleAtom) -> StyleAtom {
    match atom {
        StyleAtom::FontSize(px) => {
            let scale = f32::from_bits(current());
            StyleAtom::FontSize((f32::from(px) * scale).round().clamp(1.0, f32::from(i16::MAX)) as i16)
        }
        other => other,
    }
}
