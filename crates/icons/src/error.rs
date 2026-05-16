//! Structured error type for icon baking failures.

/// All ways an [`crate::IconBaker::bake`] call can fail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BakeError {
    /// The SVG source string could not be parsed.
    SvgParseError(String),
    /// A path segment coordinates overflow the viewBox bounds.
    PathOverflow,
    /// The total number of quadratic segments after cubic→quadratic
    /// conversion exceeded [`crate::MAX_PATH_SEGMENTS`].
    TooManySegments(u32),
    /// The SVG root has no `viewBox` attribute; baking requires one.
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
