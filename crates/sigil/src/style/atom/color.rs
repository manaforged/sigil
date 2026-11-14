use crate::style::Token;

/// An 8-bit RGBA color.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Color {
    /// The red channel, from 0 to 255.
    pub r: u8,
    /// The green channel, from 0 to 255.
    pub g: u8,
    /// The blue channel, from 0 to 255.
    pub b: u8,
    /// The alpha channel, from 0 for transparent to 255 for opaque.
    pub a: u8,
}

impl Color {
    /// Transparent black (`rgba(0, 0, 0, 0)`).
    pub const TRANSPARENT: Self = Self::rgba(0, 0, 0, 0);
    /// Opaque black (`rgb(0, 0, 0)`).
    pub const BLACK: Self = Self::rgb(0, 0, 0);
    /// Opaque white (`rgb(255, 255, 255)`).
    pub const WHITE: Self = Self::rgb(255, 255, 255);

    /// Creates an opaque color from 8-bit channels.
    #[must_use]
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self {
            r,
            g,
            b,
            a: u8::MAX,
        }
    }

    /// Creates a color from 8-bit channels and alpha.
    #[must_use]
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// Returns a copy of this color with its alpha channel set to `a`.
    #[must_use]
    pub const fn alpha(mut self, a: u8) -> Self {
        self.a = a;
        self
    }

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

impl std::str::FromStr for Color {
    type Err = crate::Error;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        parse_hex(input).ok_or_else(|| {
            crate::Error::new(
                crate::ErrorKind::Other,
                std::io::Error::new(std::io::ErrorKind::InvalidData, "hex color"),
            )
        })
    }
}

/// A literal color, a theme token, or a custom token name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ColorValue {
    /// A concrete color.
    Color(Color),
    /// A built-in theme token, resolved against the theme.
    Token(Token),
    /// A custom theme color, looked up by name.
    Custom(&'static str),
}

impl ColorValue {
    pub(crate) fn parse(name: &'static str) -> Self {
        Token::from_name(name).map_or(Self::Custom(name), Self::Token)
    }
}

impl From<Color> for ColorValue {
    fn from(value: Color) -> Self {
        Self::Color(value)
    }
}

impl From<Token> for ColorValue {
    fn from(value: Token) -> Self {
        Self::Token(value)
    }
}

impl From<&'static str> for ColorValue {
    fn from(name: &'static str) -> Self {
        Self::parse(name)
    }
}

/// The direction of a background gradient.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum GradientAxis {
    /// The first stop is at the top edge and the last at the bottom edge.
    TopToBottom,
    /// The first stop is at the bottom edge and the last at the top edge.
    BottomToTop,
    /// The first stop is at the left edge and the last at the right edge.
    LeftToRight,
    /// The first stop is at the right edge and the last at the left edge.
    RightToLeft,
    /// Toward this angle in degrees clockwise from up, like CSS `linear-gradient(<angle>)`.
    Angle(i16),
}

impl GradientAxis {
    pub(crate) fn heading(self) -> (f32, [f32; 2]) {
        match self {
            Self::BottomToTop => (0.0, [0.0, -1.0]),
            Self::LeftToRight => (90.0, [1.0, 0.0]),
            Self::TopToBottom => (180.0, [0.0, 1.0]),
            Self::RightToLeft => (270.0, [-1.0, 0.0]),
            Self::Angle(degrees) => {
                let degrees = f32::from(degrees);
                (degrees, direction(degrees))
            }
        }
    }
}

pub(crate) fn direction(degrees: f32) -> [f32; 2] {
    let (sin, cos) = degrees.to_radians().sin_cos();
    [sin, -cos]
}

fn parse_hex(input: &str) -> Option<Color> {
    let trimmed = input.trim();
    let hex = trimmed
        .strip_prefix('#')
        .or_else(|| trimmed.strip_prefix("0x"))
        .unwrap_or(trimmed);
    let digits = hex.bytes().map(nibble).collect::<Option<Vec<u8>>>()?;
    let pair = |high: u8, low: u8| high * 16 + low;
    let (r, g, b, a) = match *digits.as_slice() {
        [r, g, b] => (r * 17, g * 17, b * 17, 255),
        [r1, r0, g1, g0, b1, b0] => (pair(r1, r0), pair(g1, g0), pair(b1, b0), 255),
        [r1, r0, g1, g0, b1, b0, a1, a0] => {
            (pair(r1, r0), pair(g1, g0), pair(b1, b0), pair(a1, a0))
        }
        _ => return None,
    };
    Some(Color { r, g, b, a })
}

const fn nibble(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}
