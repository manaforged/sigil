#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BakeError {
    SvgParseError(String),
    PathOverflow,
    TooManySegments(u32),
    ViewBoxMissing,
}

impl core::fmt::Display for BakeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::SvgParseError(msg) => write!(f, "SVG parse error: {msg}"),
            Self::PathOverflow => f.write_str("path coordinates overflow viewBox bounds"),
            Self::TooManySegments(n) => {
                write!(f, "too many path segments after conversion ({n})")
            }
            Self::ViewBoxMissing => f.write_str("SVG viewBox attribute is missing"),
        }
    }
}

impl std::error::Error for BakeError {}
