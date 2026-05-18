//! Text engine error types.

use core::fmt;

/// Errors returned by the owned text engine.
///
/// The G1.4 layout path itself is infallible; font registration is the
/// only fallible surface. New fallible paths (font discovery, fallback
/// chains) add variants here without breaking the trait surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextError {
    /// Font bytes did not parse as a valid OpenType/TrueType face.
    InvalidFont,
}

impl fmt::Display for TextError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFont => f.write_str("invalid font data"),
        }
    }
}

impl std::error::Error for TextError {}
