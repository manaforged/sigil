use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextError {
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
