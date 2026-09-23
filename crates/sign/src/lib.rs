//! Release packaging for Sigil applications.
//!
//! Build a macOS `.app` or a Windows app directory, then sign update
//! packages with minisign. The envelope is the reference minisign format.

#![deny(missing_docs)]

use std::error::Error;
use std::fmt;
use std::fs::{File, read_to_string};
use std::io;
use std::path::Path;

mod cli;
mod pack;

pub use cli::pack_cli;
pub use pack::{AppBundle, CodeSign, PackError, ResourceCopy, macos_app, windows_dir};

use minisign::{PError, SecretKey, SecretKeyBox};

/// A signing failure.
#[derive(Debug)]
pub enum SignError {
    /// The key file could not be read.
    KeyFile(io::Error),
    /// The secret key could not be parsed or decrypted.
    Key(PError),
    /// The package could not be read.
    Package(io::Error),
    /// The envelope could not be produced.
    Envelope(PError),
}

impl fmt::Display for SignError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::KeyFile(error) => write!(f, "read secret key file: {error}"),
            Self::Key(error) => write!(f, "load secret key: {error}"),
            Self::Package(error) => write!(f, "read package: {error}"),
            Self::Envelope(error) => write!(f, "build envelope: {error}"),
        }
    }
}

impl Error for SignError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::KeyFile(error) | Self::Package(error) => Some(error),
            Self::Key(error) | Self::Envelope(error) => Some(error),
        }
    }
}

/// Sign the file at `package` with the secret key file at `key`.
///
/// `password` decrypts an encrypted secret key. Pass an empty string for an
/// unencrypted key. The return value is the minisign envelope as text. The
/// caller writes it. A standard `.sig` file holds the plain envelope; a
/// transport that needs an ASCII-safe payload base64-encodes this string.
///
/// # Errors
///
/// Returns [`SignError::KeyFile`] when the key file cannot be read,
/// [`SignError::Key`] when the key cannot be parsed or decrypted,
/// [`SignError::Package`] when the package cannot be read, and
/// [`SignError::Envelope`] when the signature cannot be produced.
pub fn sign_file(package: &Path, key: &Path, password: &str) -> Result<String, SignError> {
    let secret_key = read_to_string(key).map_err(SignError::KeyFile)?;
    sign_file_with_key(package, &secret_key, password)
}

/// Sign the file at `package` with `secret_key`, a minisign secret key box.
///
/// Use this when the key arrives as text rather than as a file, such as a
/// release secret that stores the box base64-encoded. Decode the transport
/// encoding before the call; this function takes the box itself.
///
/// # Errors
///
/// Returns [`SignError::Key`] when the key cannot be parsed or decrypted,
/// [`SignError::Package`] when the package cannot be read, and
/// [`SignError::Envelope`] when the signature cannot be produced.
pub fn sign_file_with_key(
    package: &Path,
    secret_key: &str,
    password: &str,
) -> Result<String, SignError> {
    let key = load_key(secret_key, password)?;
    let data = File::open(package).map_err(SignError::Package)?;
    let signature = minisign::sign(None, &key, data, None, None).map_err(SignError::Envelope)?;
    Ok(signature.into_string())
}

fn load_key(secret_key: &str, password: &str) -> Result<SecretKey, SignError> {
    SecretKeyBox::from_string(secret_key)
        .and_then(|boxed| boxed.into_secret_key(Some(password.to_owned())))
        .map_err(SignError::Key)
}

#[cfg(test)]
#[path = "sign_tests.rs"]
mod tests;
