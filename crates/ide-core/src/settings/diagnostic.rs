use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::exhaustive_enums)]
pub enum Diagnostic {
    UnknownKey {
        key: String,
        where_: PathBuf,
    },
    TypeMismatch {
        key: String,
        expected: &'static str,
        found: String,
        where_: PathBuf,
    },
}

impl Diagnostic {
    #[must_use]
    pub fn key(&self) -> &str {
        match self {
            Self::UnknownKey { key, .. } | Self::TypeMismatch { key, .. } => key,
        }
    }

    #[must_use]
    pub fn path(&self) -> &std::path::Path {
        match self {
            Self::UnknownKey { where_, .. } | Self::TypeMismatch { where_, .. } => where_,
        }
    }
}
