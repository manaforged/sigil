use crate::style::Token;

/// Semantic color family.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum Tone {
    /// Gray.
    #[default]
    Neutral,
    /// The accent color of the theme.
    Primary,
    /// Information, cyan by default.
    Info,
    /// Success, green by default.
    Success,
    /// Warning, amber by default.
    Warning,
    /// Error, red by default.
    Error,
}

impl Tone {
    /// Returns the token of this tone at `step`.
    #[must_use]
    pub const fn step(self, step: u8) -> Token {
        Token::Step(
            self,
            if step < 1 {
                1
            } else if step > 12 {
                12
            } else {
                step
            },
        )
    }

    /// The token for step 4 of the color scale of this tone.
    #[must_use]
    pub const fn shade(self) -> Token {
        self.step(4)
    }

    /// The token for step 9 of the color scale of this tone.
    #[must_use]
    pub const fn fill(self) -> Token {
        self.step(9)
    }

    /// The token for step 11 of the color scale of this tone.
    #[must_use]
    pub const fn solid(self) -> Token {
        self.step(11)
    }
}

kittest_pub! {
    /// Carries the tone that descendants resolve tone colors against.
    #[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
    pub struct ToneContext {
        /// The tone that descendants resolve tone steps against.
        pub tone: Tone,
    }
}

impl ToneContext {
    kittest_pub! {
        /// Creates a context that carries `tone`.
        #[must_use]
        pub const fn new(tone: Tone) -> Self {
            Self { tone }
        }
    }
}

impl From<Tone> for ToneContext {
    fn from(tone: Tone) -> Self {
        Self::new(tone)
    }
}

impl crate::reactive::sealed::Sealed<Self> for Tone {
    fn read(&self) -> Self {
        *self
    }

    fn into_signal(self) -> crate::reactive::Signal<Self> {
        crate::reactive::signal(self)
    }
}
