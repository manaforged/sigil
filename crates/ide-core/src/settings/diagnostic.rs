//! Non-fatal diagnostics surfaced during TOML load.
//!
//! Load errors at the *protocol* level (file unreadable, syntactically
//! invalid TOML) bubble up as `io::Error`. Per-key issues (unknown key
//! name, value wrong type for the schema entry) are diagnostics — load
//! succeeds with the value omitted and the diagnostic recorded.

use std::path::PathBuf;

/// Non-fatal issue encountered while reading a settings TOML.
///
/// The `where_` field gives the file path the issue originated in so
/// editors can surface it in the right buffer.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::exhaustive_enums)]
pub enum Diagnostic {
    /// TOML file contained a key that the [`crate::settings::Schema`] did
    /// not register. The store ignores it; this diagnostic is the only
    /// trace.
    UnknownKey {
        /// The TOML key name (dotted, as written in the file).
        key: String,
        /// Path to the settings file that contained the key.
        where_: PathBuf,
    },
    /// TOML value did not match the schema-declared type for this key.
    TypeMismatch {
        /// Schema key name.
        key: String,
        /// Schema-declared type (e.g. `"integer"`).
        expected: &'static str,
        /// Type as read from TOML (e.g. `"string"`).
        found: String,
        /// Path to the settings file that contained the bad value.
        where_: PathBuf,
    },
}

impl Diagnostic {
    /// The TOML key the diagnostic refers to.
    #[must_use]
    pub fn key(&self) -> &str {
        match self {
            Self::UnknownKey { key, .. } | Self::TypeMismatch { key, .. } => key,
        }
    }

    /// The file path the diagnostic came from.
    #[must_use]
    pub fn path(&self) -> &std::path::Path {
        match self {
            Self::UnknownKey { where_, .. } | Self::TypeMismatch { where_, .. } => where_,
        }
    }
}
