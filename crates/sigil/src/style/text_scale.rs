use core::sync::atomic::{AtomicU32, Ordering};

use crate::style::StyleAtom;

static SCALE: AtomicU32 = AtomicU32::new(0x3F80_0000);

/// Scales the named text sizes (`text-xs` to `text-xl`); 1.0 keeps them as they are.
pub fn text_scale(scale: f32) {
    let scale = if scale > 0.0 { scale } else { 1.0 };
    SCALE.store(scale.to_bits(), Ordering::Relaxed);
}

/// A step of the type scale: size, line height, and weight, set together.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TextRole {
    /// 12 px text for secondary detail.
    Caption,
    /// 14 px running text.
    Body,
    /// 14 px medium text for controls and form labels.
    Label,
    /// 16 px semibold text for a section.
    Subtitle,
    /// 20 px semibold text for a panel or dialog.
    Title,
    /// 24 px semibold text for a page.
    Heading,
    /// 32 px bold text for a hero.
    Display,
}

impl TextRole {
    pub(crate) fn atoms(self) -> [StyleAtom; 3] {
        let (size, line, weight) = match self {
            Self::Caption => (12, 16, 400),
            Self::Body => (14, 20, 400),
            Self::Label => (14, 20, 500),
            Self::Subtitle => (16, 24, 600),
            Self::Title => (20, 28, 600),
            Self::Heading => (24, 32, 600),
            Self::Display => (32, 40, 700),
        };
        let line = match scaled(StyleAtom::FontSize(line)) {
            StyleAtom::FontSize(px) => px,
            _ => line,
        };
        [
            scaled(StyleAtom::FontSize(size)),
            StyleAtom::LineHeight(line),
            StyleAtom::FontWeight(weight),
        ]
    }
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
            StyleAtom::FontSize(
                (f32::from(px) * scale)
                    .round()
                    .clamp(1.0, f32::from(i16::MAX)) as i16,
            )
        }
        other => other,
    }
}
