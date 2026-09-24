use std::error::Error;
use std::fmt;
use std::fs::{File, read_to_string};
use std::io;
use std::path::Path;

mod cli;
mod files;
mod pack;

pub use cli::pack_cli;
pub use pack::{AppBundle, CodeSign, PackError, ResourceCopy, macos_app, windows_dir};

use minisign::{PError, PublicKey, SecretKey, SecretKeyBox, SignatureBox};

#[derive(Debug)]
#[non_exhaustive]
pub enum SignError {
    KeyFile(io::Error),
    Key(PError),
    Package(io::Error),
    Envelope(PError),
    TrustedComment(String),
}

impl fmt::Display for SignError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::KeyFile(error) => write!(f, "read secret key file: {error}"),
            Self::Key(error) => write!(f, "load key: {error}"),
            Self::Package(error) => write!(f, "read package: {error}"),
            Self::Envelope(error) => write!(f, "signature envelope: {error}"),
            Self::TrustedComment(reason) => write!(f, "trusted comment: {reason}"),
        }
    }
}

impl Error for SignError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::KeyFile(error) | Self::Package(error) => Some(error),
            Self::Key(error) | Self::Envelope(error) => Some(error),
            Self::TrustedComment(_) => None,
        }
    }
}

pub fn sign_file(
    package: &Path,
    key: &Path,
    password: Option<&str>,
    version: &str,
) -> Result<String, SignError> {
    let secret_key = read_to_string(key).map_err(SignError::KeyFile)?;
    sign_file_with_key(package, &secret_key, password, version)
}

pub fn sign_file_with_key(
    package: &Path,
    secret_key: &str,
    password: Option<&str>,
    version: &str,
) -> Result<String, SignError> {
    let key = load_key(secret_key, password)?;
    let comment = format!("file:{}\tversion:{version}", file_name(package)?);
    let data = File::open(package).map_err(SignError::Package)?;
    let signature =
        minisign::sign(None, &key, data, Some(&comment), None).map_err(SignError::Envelope)?;
    Ok(signature.into_string())
}

pub fn verify_file(
    public_key: &Path,
    file: &Path,
    sig: &Path,
    expected_version: Option<&str>,
) -> Result<String, SignError> {
    let key = PublicKey::from_file(public_key).map_err(SignError::Key)?;
    let signature = SignatureBox::from_file(sig).map_err(SignError::Envelope)?;
    let data = File::open(file).map_err(SignError::Package)?;
    minisign::verify(&key, &signature, data, true, false, false).map_err(SignError::Envelope)?;
    let comment = signature.trusted_comment().map_err(SignError::Envelope)?;
    let name = comment_field(&comment, "file")?;
    if name != file_name(file)? {
        return Err(SignError::TrustedComment(format!(
            "signed for file {name}, not {}",
            file.display()
        )));
    }
    let version = comment_field(&comment, "version")?;
    match expected_version {
        Some(expected) if expected != version => Err(SignError::TrustedComment(format!(
            "signed for version {version}, expected {expected}"
        ))),
        _ => Ok(version.to_owned()),
    }
}

fn comment_field<'a>(comment: &'a str, key: &str) -> Result<&'a str, SignError> {
    comment
        .split('\t')
        .find_map(|field| field.strip_prefix(key)?.strip_prefix(':'))
        .ok_or_else(|| SignError::TrustedComment(format!("no {key} field in {comment}")))
}

fn file_name(path: &Path) -> Result<&str, SignError> {
    path.file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            SignError::Package(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("{} has no UTF-8 file name", path.display()),
            ))
        })
}

fn load_key(secret_key: &str, password: Option<&str>) -> Result<SecretKey, SignError> {
    let password = password.unwrap_or_default().to_owned();
    SecretKeyBox::from_string(secret_key)
        .and_then(|boxed| boxed.into_secret_key(Some(password)))
        .map_err(SignError::Key)
}

#[cfg(test)]
#[path = "sign_tests.rs"]
mod tests;
