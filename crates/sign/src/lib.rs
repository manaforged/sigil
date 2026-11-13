#![doc = include_str!("../README.md")]
#![deny(missing_docs)]
use std::fs::{File, read_to_string};
use std::io;
use std::path::Path;

mod cli;
#[cfg(target_os = "macos")]
mod dmg;
mod error;
mod files;
mod pack;

pub use cli::pack_cli;
#[cfg(target_os = "macos")]
pub use dmg::{Dmg, macos_dmg, notarize};
pub use error::{Error, ErrorKind};
pub use pack::{AppBundle, CodeSign, ResourceCopy, macos_app, windows_dir};

use minisign::{PError, PublicKey, SecretKey, SecretKeyBox, SignatureBox};

/// Signs a package with the secret key file at `key` and returns the minisign envelope.
pub fn sign_file(
    package: &Path,
    key: &Path,
    password: Option<&str>,
    version: &str,
) -> Result<String, Error> {
    let secret_key =
        read_to_string(key).map_err(|error| Error::caused(ErrorKind::KeyFile, error))?;
    sign_file_with_key(package, &secret_key, password, version)
}

/// Signs a package with a secret key box held as text and returns the minisign envelope.
pub fn sign_file_with_key(
    package: &Path,
    secret_key: &str,
    password: Option<&str>,
    version: &str,
) -> Result<String, Error> {
    let key = load_key(secret_key, password)?;
    let comment = format!("file:{}\tversion:{version}", file_name(package)?);
    let data = File::open(package).map_err(package_error)?;
    let signature =
        minisign::sign(None, &key, data, Some(&comment), None).map_err(envelope_error)?;
    Ok(signature.into_string())
}

/// Verifies a package against its signature, its file name, and the expected version.
pub fn verify_file(
    public_key: &Path,
    file: &Path,
    sig: &Path,
    expected_version: &str,
) -> Result<(), Error> {
    let key = PublicKey::from_file(public_key).map_err(key_error)?;
    let signature = SignatureBox::from_file(sig).map_err(envelope_error)?;
    let data = File::open(file).map_err(package_error)?;
    minisign::verify(&key, &signature, data, true, false, false).map_err(envelope_error)?;
    let comment = signature.trusted_comment().map_err(envelope_error)?;
    let name = comment_field(&comment, "file")?;
    if name != file_name(file)? {
        return Err(Error::text(
            ErrorKind::TrustedComment,
            format!("signed for file {name}, not {}", file.display()),
        ));
    }
    let version = comment_field(&comment, "version")?;
    if version != expected_version {
        return Err(Error::text(
            ErrorKind::TrustedComment,
            format!("signed for version {version}, expected {expected_version}"),
        ));
    }
    Ok(())
}

fn key_error(error: PError) -> Error {
    Error::caused(ErrorKind::Key, error)
}

fn envelope_error(error: PError) -> Error {
    Error::caused(ErrorKind::Envelope, error)
}

fn package_error(error: io::Error) -> Error {
    Error::caused(ErrorKind::Package, error)
}

fn comment_field<'a>(comment: &'a str, key: &str) -> Result<&'a str, Error> {
    comment
        .split('\t')
        .find_map(|field| field.strip_prefix(key)?.strip_prefix(':'))
        .ok_or_else(|| {
            Error::text(
                ErrorKind::TrustedComment,
                format!("no {key} field in {comment}"),
            )
        })
}

fn file_name(path: &Path) -> Result<&str, Error> {
    path.file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            package_error(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("{} has no UTF-8 file name", path.display()),
            ))
        })
}

fn load_key(secret_key: &str, password: Option<&str>) -> Result<SecretKey, Error> {
    let boxed = SecretKeyBox::from_string(secret_key).map_err(key_error)?;
    boxed
        .into_secret_key(Some(password.unwrap_or_default().to_owned()))
        .map_err(|error| match password {
            None => Error::fixed(ErrorKind::PasswordRequired),
            Some(_) => key_error(error),
        })
}

#[cfg(test)]
#[path = "sign_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "pack_native_tests.rs"]
mod pack_native_tests;

#[cfg(test)]
#[path = "pack_profile_tests.rs"]
mod pack_profile_tests;

#[cfg(doctest)]
#[doc = include_str!("../../../docs/guide/signing.md")]
#[derive(Debug)]
pub struct SigningGuide;
