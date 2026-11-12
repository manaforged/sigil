/// Font family for `font_family`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum FontFamily {
    /// The sans font family.
    Sans,
    /// The mono font family.
    Mono,
    /// The serif font family.
    Serif,
    /// The icon font family.
    Icon,
    /// The display font family.
    Display,
}

impl FontFamily {
    /// The lowercase name of this family, such as `sans` or `mono`.
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
