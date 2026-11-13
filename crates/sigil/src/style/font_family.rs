/// Font family for `font_family`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum FontFamily {
    /// The sans-serif family, named `sans`.
    Sans,
    /// The fixed-width family, named `mono`.
    Mono,
    /// The serif family, named `serif`; text falls back to sans when no serif font is loaded.
    Serif,
    /// The family of icon glyphs, named `icon`; text falls back to sans when no icon font is loaded.
    Icon,
    /// The display family, named `display`; text falls back to sans when no display font is loaded.
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
