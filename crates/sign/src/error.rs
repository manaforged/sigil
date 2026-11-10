use std::error::Error as StdError;
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

type Source = Box<dyn StdError + Send + Sync + 'static>;

/// Class of an `Error`.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorKind {
    /// The secret key file could not be read.
    KeyFile,
    /// A key could not be loaded or generated.
    Key,
    /// The package could not be read.
    Package,
    /// The signature envelope is invalid or does not match the package.
    Envelope,
    /// The trusted comment does not name the expected file or version.
    TrustedComment,
    /// The secret key is encrypted and no password was given.
    PasswordRequired,
    /// A required file or directory is missing.
    Missing,
    /// A file operation failed.
    Io,
    /// Native code signing failed or was refused.
    Codesign,
    /// A platform tool exited with a failure.
    Tool,
    /// A path leaves the bundle.
    Escape,
    /// A path is a symlink.
    Symlink,
    /// The destination holds files that the packer did not write.
    Occupied,
    /// A path is not a regular file.
    NotFile,
    /// The command line is invalid.
    Usage,
}

impl ErrorKind {
    const fn label(self) -> &'static str {
        match self {
            Self::KeyFile => "read secret key file",
            Self::Key => "load key",
            Self::Package => "read package",
            Self::Envelope => "signature envelope",
            Self::TrustedComment => "trusted comment",
            Self::PasswordRequired => "the secret key is encrypted; give its password",
            Self::Missing => "missing",
            Self::Io => "file operation failed",
            Self::Codesign => "codesign",
            Self::Tool => "tool failed",
            Self::Escape => "path escapes bundle",
            Self::Symlink => "symlink refused",
            Self::Occupied => "refusing to delete",
            Self::NotFile => "not a regular file",
            Self::Usage => "invalid command line",
        }
    }
}

#[derive(Debug)]
enum Detail {
    Kind,
    Bare(String),
    Text(String),
    Path(PathBuf),
}

/// Failure of a signing, packing, or command-line step.
#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
    detail: Detail,
    source: Option<Source>,
}

impl Error {
    pub(crate) const fn fixed(kind: ErrorKind) -> Self {
        Self {
            kind,
            detail: Detail::Kind,
            source: None,
        }
    }

    pub(crate) fn caused(kind: ErrorKind, source: impl StdError + Send + Sync + 'static) -> Self {
        Self {
            kind,
            detail: Detail::Kind,
            source: Some(Box::new(source)),
        }
    }

    pub(crate) fn io(source: io::Error) -> Self {
        Self::caused(ErrorKind::Io, source)
    }

    pub(crate) fn text(kind: ErrorKind, text: impl Into<String>) -> Self {
        Self {
            kind,
            detail: Detail::Text(text.into()),
            source: None,
        }
    }

    pub(crate) fn bare(kind: ErrorKind, text: impl Into<String>) -> Self {
        Self {
            kind,
            detail: Detail::Bare(text.into()),
            source: None,
        }
    }

    pub(crate) fn context(
        kind: ErrorKind,
        text: impl Into<String>,
        source: impl StdError + Send + Sync + 'static,
    ) -> Self {
        Self {
            kind,
            detail: Detail::Bare(text.into()),
            source: Some(Box::new(source)),
        }
    }

    pub(crate) fn usage(text: impl Into<String>) -> Self {
        Self::bare(ErrorKind::Usage, text)
    }

    pub(crate) fn at(kind: ErrorKind, path: &Path) -> Self {
        Self {
            kind,
            detail: Detail::Path(path.to_path_buf()),
            source: None,
        }
    }

    /// Returns the kind.
    #[must_use]
    pub const fn kind(&self) -> ErrorKind {
        self.kind
    }

    /// Returns the path that the error is about.
    #[must_use]
    pub fn path(&self) -> Option<&Path> {
        match &self.detail {
            Detail::Path(path) => Some(path),
            _ => None,
        }
    }

    /// Reports whether the secret key is encrypted and no password was given.
    #[must_use]
    pub const fn is_password_required(&self) -> bool {
        matches!(self.kind, ErrorKind::PasswordRequired)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = self.kind.label();
        match &self.detail {
            Detail::Kind => f.write_str(label),
            Detail::Bare(text) => f.write_str(text),
            Detail::Text(text) => write!(f, "{label}: {text}"),
            Detail::Path(path) => match self.kind {
                ErrorKind::Missing | ErrorKind::Occupied => {
                    write!(f, "{label} {}", path.display())
                }
                _ => write!(f, "{label}: {}", path.display()),
            },
        }
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        let source = self.source.as_deref()?;
        Some(source)
    }
}
