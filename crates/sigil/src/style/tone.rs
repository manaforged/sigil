use crate::style::Token;
use crate::style::scale::ScaleRole;

/// Semantic color family.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum Tone {
    /// The neutral tone.
    #[default]
    Neutral,
    /// The primary tone.
    Primary,
    /// The info tone.
    Info,
    /// The success tone.
    Success,
    /// The warning tone.
    Warning,
    /// The error tone.
    Error,
}

impl Tone {
    /// Returns the token of this tone at `step`.
    #[must_use]
    pub fn token(self, step: u8) -> Token {
        Token::scale(self.scale_role(), step).unwrap_or(Token::OnSurface)
    }

    /// Returns the shade.
    #[must_use]
    pub fn shade(self) -> Token {
        self.token(4)
    }

    /// Returns the fill.
    #[must_use]
    pub fn fill(self) -> Token {
        self.token(9)
    }

    /// Returns the solid.
    #[must_use]
    pub fn solid(self) -> Token {
        self.token(11)
    }

    /// Returns the scale role.
    #[must_use]
    pub const fn scale_role(self) -> ScaleRole {
        match self {
            Self::Neutral => ScaleRole::Gray,
            Self::Primary => ScaleRole::Accent,
            Self::Info => ScaleRole::Info,
            Self::Success => ScaleRole::Success,
            Self::Warning => ScaleRole::Warning,
            Self::Error => ScaleRole::Error,
        }
    }
}

/// Carries the tone that descendants resolve tone colors against.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct ToneContext {
    /// The tone.
    pub tone: Tone,
}

impl ToneContext {
    /// Creates a tone context.
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
