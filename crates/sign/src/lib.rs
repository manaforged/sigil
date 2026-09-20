//! Release packaging for Sigil applications.
//!
//! Build a macOS `.app` or a Windows app directory, then sign update
//! packages with minisign. The envelope is the reference minisign format.

#![deny(missing_docs)]

use std::error::Error;
use std::fmt;
use std::fs::File;
use std::io;
use std::path::Path;

mod cli;
mod pack;

pub use cli::pack_cli;
pub use pack::{AppBundle, PackError, ResourceCopy, macos_app, windows_dir};

use minisign::{PError, SecretKey};

/// A signing failure.
#[derive(Debug)]
pub enum SignError {
    /// The secret key could not be read or decrypted.
    Key(PError),
    /// The package could not be read.
    Package(io::Error),
    /// The envelope could not be produced.
    Envelope(PError),
}

impl fmt::Display for SignError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Key(error) => write!(f, "load secret key: {error}"),
            Self::Package(error) => write!(f, "read package: {error}"),
            Self::Envelope(error) => write!(f, "build envelope: {error}"),
        }
    }
}

impl Error for SignError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Key(error) | Self::Envelope(error) => Some(error),
            Self::Package(error) => Some(error),
        }
    }
}

/// Sign the file at `package` with the secret key at `key`.
///
/// `password` decrypts an encrypted secret key. Pass an empty string for an
/// unencrypted key. The return value is the minisign envelope as text. The
/// caller writes it wherever the feed expects it. A standard `.sig` file
/// holds the plain envelope; a transport that needs an ASCII-safe payload
/// base64-encodes this string.
///
/// # Errors
///
/// Returns [`SignError::Key`] when the key cannot be read or decrypted,
/// [`SignError::Package`] when the package cannot be read, and
/// [`SignError::Envelope`] when the signature cannot be produced.
pub fn sign_file(package: &Path, key: &Path, password: &str) -> Result<String, SignError> {
    let secret_key = load_key(key, password)?;
    let data = File::open(package).map_err(SignError::Package)?;
    let signature =
        minisign::sign(None, &secret_key, data, None, None).map_err(SignError::Envelope)?;
    Ok(signature.into_string())
}

fn load_key(key: &Path, password: &str) -> Result<SecretKey, SignError> {
    SecretKey::from_file(key, Some(password.to_owned())).map_err(SignError::Key)
}

#[cfg(test)]
#[path = "sign_tests.rs"]
mod tests;
