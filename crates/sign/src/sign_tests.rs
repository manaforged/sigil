use std::error::Error;
use std::fs;
use std::path::PathBuf;

use super::{sign_file, sign_file_with_key};

const PASSWORD: &str = "sigil-sign-test";
const FIXTURE: &[u8] = b"sigil signing fixture";

struct Fixture {
    dir: PathBuf,
    key: PathBuf,
    package: PathBuf,
    pair: minisign::KeyPair,
}

impl Fixture {
    fn new(name: &str, bytes: &[u8]) -> Result<Self, Box<dyn Error>> {
        let dir = std::env::temp_dir().join(format!("sigil-sign-{}-{name}", std::process::id()));
        fs::create_dir_all(&dir)?;
        let pair = minisign::KeyPair::generate_encrypted_keypair(Some(PASSWORD.to_owned()))?;
        let key = dir.join("release.key");
        fs::write(&key, pair.sk.to_box(None)?.into_string())?;
        let package = dir.join("package.nupkg");
        fs::write(&package, bytes)?;
        Ok(Self {
            dir,
            key,
            package,
            pair,
        })
    }

    fn verify(&self, envelope: &str) -> Result<(), Box<dyn Error>> {
        let signature = minisign::SignatureBox::from_string(envelope)?;
        let data = fs::File::open(&self.package)?;
        minisign::verify(&self.pair.pk, &signature, data, true, false, true)?;
        Ok(())
    }

    fn clean(self) -> Result<(), Box<dyn Error>> {
        fs::remove_dir_all(&self.dir)?;
        Ok(())
    }
}

#[test]
fn sign_then_verify_round_trips() -> Result<(), Box<dyn Error>> {
    let fixture = Fixture::new("round-trip", FIXTURE)?;
    let envelope = sign_file(&fixture.package, &fixture.key, PASSWORD)?;
    assert!(envelope.starts_with("untrusted comment:"));
    fixture.verify(&envelope)?;
    fixture.clean()?;
    Ok(())
}

#[test]
fn a_key_held_as_text_signs_the_same_package() -> Result<(), Box<dyn Error>> {
    let fixture = Fixture::new("key-as-text", FIXTURE)?;
    let key = fs::read_to_string(&fixture.key)?;
    let envelope = sign_file_with_key(&fixture.package, &key, PASSWORD)?;
    fixture.verify(&envelope)?;
    fixture.clean()?;
    Ok(())
}

#[test]
fn tampered_package_fails_verification() -> Result<(), Box<dyn Error>> {
    let fixture = Fixture::new("tampered", FIXTURE)?;
    let envelope = sign_file(&fixture.package, &fixture.key, PASSWORD)?;
    fs::write(&fixture.package, b"sigil signing fixture ")?;
    assert!(fixture.verify(&envelope).is_err());
    fixture.clean()?;
    Ok(())
}

#[test]
fn wrong_password_fails_to_load_the_key() -> Result<(), Box<dyn Error>> {
    let fixture = Fixture::new("wrong-password", FIXTURE)?;
    assert!(sign_file(&fixture.package, &fixture.key, "not the password").is_err());
    fixture.clean()?;
    Ok(())
}
