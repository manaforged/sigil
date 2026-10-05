use crate::style::Token;
use crate::style::scale::ScaleRole;

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum Tone {
    #[default]
    Neutral,
    Primary,
    Info,
    Success,
    Warn,
    Error,
}

impl Tone {
    #[must_use]
    pub fn token(self, step: u8) -> Token {
        Token::scale(self.scale_role(), step).unwrap_or(Token::OnSurface)
    }

    #[must_use]
    pub fn shade(self) -> Token {
        self.token(4)
    }

    #[must_use]
    pub fn fill(self) -> Token {
        self.token(9)
    }

    #[must_use]
    pub fn solid(self) -> Token {
        self.token(11)
    }

    #[must_use]
    pub const fn scale_role(self) -> ScaleRole {
        match self {
            Self::Neutral => ScaleRole::Gray,
            Self::Primary => ScaleRole::Accent,
            Self::Info => ScaleRole::Info,
            Self::Success => ScaleRole::Success,
            Self::Warn => ScaleRole::Warn,
            Self::Error => ScaleRole::Error,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct ToneContext {
    pub tone: Tone,
}

impl ToneContext {
    #[must_use]
    pub const fn new(tone: Tone) -> Self {
        Self { tone }
    }
}

impl From<Tone> for ToneContext {
    fn from(tone: Tone) -> Self {
        Self::new(tone)
    }
}

impl crate::reactive::Reactive<Self> for Tone {
    fn read(&self) -> Self {
        *self
    }

    fn into_signal(self) -> crate::reactive::Signal<Self> {
        crate::reactive::signal(self)
    }
}
