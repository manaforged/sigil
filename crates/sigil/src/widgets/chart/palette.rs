use crate::style::{Color, ColorValue, StyleAtom, Token, Tone};

const FILL: u8 = 9;
const SOFT_DROP: u8 = 5;
const SOFT_ALPHA: u8 = 0x70;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct Hue {
    pub(super) tone: Tone,
    step: u8,
    rgb: Option<Color>,
}

const PALETTE: [Hue; 10] = [
    Hue::new(Tone::Primary, 9),
    Hue::new(Tone::Success, 9),
    Hue::new(Tone::Warning, 9),
    Hue::new(Tone::Error, 9),
    Hue::new(Tone::Info, 9),
    Hue::new(Tone::Primary, 7),
    Hue::new(Tone::Success, 7),
    Hue::new(Tone::Warning, 7),
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

    pub(super) const fn rgb(color: Color) -> Self {
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

    pub(super) fn color(self) -> ColorValue {
        self.rgb
            .map_or_else(|| ColorValue::Token(self.token()), ColorValue::Color)
    }

    pub(super) fn background(self) -> StyleAtom {
        self.rgb.map_or_else(
            || self.token().background(),
            |color| StyleAtom::BackgroundColor(ColorValue::Color(color)),
        )
    }

    pub(super) fn soft(self) -> ColorValue {
        self.rgb.map_or_else(
            || ColorValue::Token(self.tone.token(self.step.saturating_sub(SOFT_DROP).max(2))),
            |color| {
                ColorValue::Color(Color {
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
