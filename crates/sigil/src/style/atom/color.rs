use crate::style::Token;

/// An 8-bit RGBA color.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rgba8 {
    /// The r.
    pub r: u8,
    /// The g.
    pub g: u8,
    /// The b.
    pub b: u8,
    /// The a.
    pub a: u8,
}

impl Rgba8 {
    /// Returns this color drawn over the opaque `under` with straight alpha, as an opaque color.
    #[must_use]
    pub(crate) fn over(self, under: Self) -> Self {
        let alpha = u16::from(self.a);
        let mix = |top: u8, bottom: u8| {
            u8::try_from((u16::from(top) * alpha + u16::from(bottom) * (255 - alpha) + 127) / 255)
                .unwrap_or(u8::MAX)
        };
        Self {
            r: mix(self.r, under.r),
            g: mix(self.g, under.g),
            b: mix(self.b, under.b),
            a: 255,
        }
    }
}

impl std::str::FromStr for Rgba8 {
    type Err = crate::Error;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        crate::widgets::parse_color_hex(input).ok_or_else(|| {
            crate::Error::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "hex color",
            ))
        })
    }
}

/// A literal color, a theme token, or a custom token name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ColorAtom {
    /// The rgba8 color atom.
    Rgba8(Rgba8),
    /// The token color atom.
    Token(Token),
    /// The custom color atom.
    Custom(&'static str),
}

impl ColorAtom {
    pub(crate) fn parse(name: &'static str) -> Self {
        Token::from_name(name).map_or(Self::Custom(name), Self::Token)
    }
}

impl From<Rgba8> for ColorAtom {
    fn from(value: Rgba8) -> Self {
        Self::Rgba8(value)
    }
}

impl From<Token> for ColorAtom {
    fn from(value: Token) -> Self {
        Self::Token(value)
    }
}

/// The direction of a background gradient.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GradientAxis {
    /// The top to bottom gradient axis.
    TopToBottom,
    /// The bottom to top gradient axis.
    BottomToTop,
    /// The left to right gradient axis.
    LeftToRight,
    /// The right to left gradient axis.
    RightToLeft,
}

/// A color and offset of a background gradient.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GradientStopKey {
    /// The offset.
    pub offset: u16,
    /// The color.
    pub color: ColorAtom,
}

impl GradientStopKey {
    /// Creates a gradient stop key.
    #[must_use]
    pub const fn new(offset: u16, color: ColorAtom) -> Self {
        Self { offset, color }
    }
}

/// A background gradient in a style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BgGradient {
    /// The axis.
    pub axis: GradientAxis,
    /// The stops.
    pub stops: &'static [GradientStopKey],
}

impl BgGradient {
    /// Creates a bg gradient.
    #[must_use]
    pub const fn new(axis: GradientAxis, stops: &'static [GradientStopKey]) -> Self {
        Self { axis, stops }
    }

    /// Creates a gradient from owned stops.
    #[must_use]
    pub fn owned(axis: GradientAxis, stops: Vec<GradientStopKey>) -> Self {
        Self::new(axis, crate::style::intern::intern(stops))
    }
}
