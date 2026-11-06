use crate::style::{Token, Tone};

const FILL: u8 = 9;
const SOFT_DROP: u8 = 5;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct Hue {
    pub(super) tone: Tone,
    step: u8,
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
        Self { tone, step }
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

    pub(super) fn soft(self) -> Token {
        self.tone.token(self.step.saturating_sub(SOFT_DROP).max(2))
    }
}
