use std::error::Error;
use std::fs;
use std::path::PathBuf;

use super::{SignError, pack_cli, sign_file, sign_file_with_key};

const PASSWORD: &str = "sigil-sign-test";
const FIXTURE: &[u8] = b"sigil signing fixture";

struct Fixture {
    dir: PathBuf,
    key: PathBuf,
    package: PathBuf,
    pair: minisign::KeyPair,
}

impl Fixture {
    fn new(name: &str) -> Result<Self, Box<dyn Error>> {
        let dir = std::env::temp_dir().join(format!("sigil-sign-{}-{name}", std::process::id()));
        fs::create_dir_all(&dir)?;
        let pair = minisign::KeyPair::generate_encrypted_keypair(Some(PASSWORD.to_owned()))?;
        let key = dir.join("release.key");
        fs::write(&key, pair.sk.to_box(None)?.into_string())?;
        let package = dir.join("package.nupkg");
        fs::write(&package, FIXTURE)?;
        Ok(Self {
            dir,
            key,
            package,
            pair,
        })
    }

    fn sign(&self, password: Option<&str>) -> Result<String, SignError> {
        sign_file(&self.package, &self.key, password, "0.1.0")
    }

    fn verify(&self, envelope: &str) -> Result<(), Box<dyn Error>> {
        let signature = minisign::SignatureBox::from_string(envelope)?;
        let data = fs::File::open(&self.package)?;
        minisign::verify(&self.pair.pk, &signature, data, true, false, true)?;
        Ok(())
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        drop(fs::remove_dir_all(&self.dir));
    }
}

#[test]
fn sign_then_verify_round_trips() -> Result<(), Box<dyn Error>> {
    let fixture = Fixture::new("round-trip")?;
    let envelope = fixture.sign(Some(PASSWORD))?;
    assert!(envelope.starts_with("untrusted comment:"));
    fixture.verify(&envelope)
}

#[test]
fn a_key_held_as_text_signs_the_same_package() -> Result<(), Box<dyn Error>> {
    let fixture = Fixture::new("key-as-text")?;
    let key = fs::read_to_string(&fixture.key)?;
    let envelope = sign_file_with_key(&fixture.package, &key, Some(PASSWORD), "0.1.0")?;
    fixture.verify(&envelope)
}

#[test]
fn tampered_package_fails_verification() -> Result<(), Box<dyn Error>> {
    let fixture = Fixture::new("tampered")?;
    let envelope = fixture.sign(Some(PASSWORD))?;
    fs::write(&fixture.package, b"sigil signing fixture ")?;
    assert!(fixture.verify(&envelope).is_err());
    Ok(())
}

#[test]
fn a_wrong_or_missing_password_fails_to_load_the_key() -> Result<(), Box<dyn Error>> {
    let fixture = Fixture::new("password")?;
    assert!(matches!(
        fixture.sign(Some("not the password")),
        Err(SignError::Key(_))
    ));
    assert!(matches!(
        fixture.sign(None),
        Err(SignError::PasswordRequired)
    ));
    Ok(())
}

#[test]
fn verify_refuses_to_run_without_an_expected_version() -> Result<(), Box<dyn Error>> {
    let fixture = Fixture::new("no-version")?;
    fs::write(
        fixture.dir.join("package.nupkg.sig"),
        fixture.sign(Some(PASSWORD))?,
    )?;
    let public = fixture.dir.join("release.pub");
    fs::write(&public, fixture.pair.pk.to_box()?.to_string())?;
    let argv = [
        "verify",
        "--pub",
        public.to_str().ok_or("path")?,
        "--file",
        fixture.package.to_str().ok_or("path")?,
    ]
    .map(str::to_owned);
    assert_eq!(pack_cli(&argv), Err(String::from("missing --version")));
    Ok(())
}
