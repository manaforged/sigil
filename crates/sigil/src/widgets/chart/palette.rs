use super::super::vector::PathColor;
use crate::style::{ColorAtom, Rgba8, StyleAtom, Token, Tone};

const FILL: u8 = 9;
const SOFT_DROP: u8 = 5;
const SOFT_ALPHA: u8 = 0x70;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct Hue {
    pub(super) tone: Tone,
    step: u8,
    rgb: Option<Rgba8>,
}

const PALETTE: [Hue; 10] = [
    Hue::new(Tone::Primary, 9),
    Hue::new(Tone::Success, 9),
    Hue::new(Tone::Warn, 9),
    Hue::new(Tone::Error, 9),
    Hue::new(Tone::Info, 9),
    Hue::new(Tone::Primary, 7),
    Hue::new(Tone::Success, 7),
    Hue::new(Tone::Warn, 7),
    Hue::new(Tone::Error, 7),
    Hue::new(Tone::Info, 7),
];

impl Hue {
    const fn new(tone: Tone, step: u8) -> Self {
        Self {
            tone,
            step,
            rgb: None,
        }
    }

    pub(super) const fn rgb(color: Rgba8) -> Self {
        Self {
            tone: Tone::Primary,
            step: FILL,
            rgb: Some(color),
        }
    }

    pub(super) const fn of(tone: Tone) -> Self {
        Self::new(tone, FILL)
    }

    pub(super) fn nth(index: usize) -> Self {
        PALETTE
            .get(index % PALETTE.len())
            .copied()
            .unwrap_or(Self::of(Tone::Primary))
    }

    pub(super) fn token(self) -> Token {
        self.tone.token(self.step)
    }

    pub(super) fn color(self) -> ColorAtom {
        self.rgb
            .map_or(ColorAtom::Token(self.token()), ColorAtom::Rgba8)
    }

    pub(super) fn paint(self) -> PathColor {
        self.rgb
            .map_or(PathColor::Token(self.token()), PathColor::Rgba)
    }

    pub(super) fn bg(self) -> StyleAtom {
        match self.rgb {
            Some(color) => StyleAtom::BackgroundColor(ColorAtom::Rgba8(color)),
            None => self.tone.token(self.step).bg(),
        }
    }

    pub(super) fn soft(self) -> PathColor {
        self.rgb.map_or_else(
            || PathColor::Token(self.tone.token(self.step.saturating_sub(SOFT_DROP).max(2))),
            |color| {
                PathColor::Rgba(Rgba8 {
                    a: SOFT_ALPHA,
                    ..color
                })
            },
        )
    }
}

#[cfg(feature = "scene")]
pub(crate) fn series_tokens() -> impl Iterator<Item = Token> {
    PALETTE.into_iter().map(Hue::token)
}
