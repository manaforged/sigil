#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FontFamily {
    Sans,
    Mono,
    Serif,
    Icon,
    Display,
}

impl FontFamily {
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Sans => "sans",
            Self::Mono => "mono",
            Self::Serif => "serif",
            Self::Icon => "icon",
            Self::Display => "display",
        }
    }
}
