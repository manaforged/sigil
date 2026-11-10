use std::error::Error;
use std::fs;
use std::path::Path;

use super::{AppBundle, CodeSign, ErrorKind, macos_app, pack_cli, windows_dir};

fn app<'a>(executable: &'a Path, profile: &'a Path) -> AppBundle<'a> {
    AppBundle::new("Profile fixture", executable)
        .identifier("example.sigil.profile")
        .version("1.0.0")
        .exe_name("fixture")
        .provisioning_profile(profile)
}

#[test]
fn cli_embeds_exact_profile_bytes_before_native_signing() -> Result<(), Box<dyn Error>> {
    let directory = std::env::temp_dir().join(format!("sigil-profile-{}", std::process::id()));
    fs::create_dir_all(&directory)?;
    let profile = directory.join("profile");
    let bundle = directory.join("Fixture.app");
    fs::write(&profile, b"opaque provisioning profile fixture\x00\xff")?;
    let result = pack_cli(
        &[
            "macos",
            "--exe",
            std::env::current_exe()?.to_str().ok_or("executable path")?,
            "--out",
            bundle.to_str().ok_or("bundle path")?,
            "--name",
            "Fixture",
            "--id",
            "example.sigil.profile",
            "--version",
            "1.0.0",
            "--identity",
            "sigil-profile-fixture-identity-does-not-exist",
            "--provisioning-profile",
            profile.to_str().ok_or("profile path")?,
        ]
        .map(str::to_owned),
    );
    if cfg!(target_os = "macos") {
        assert!(result.is_err_and(|error| error.to_string().starts_with("codesign:")));
    } else {
        result?;
    }
    assert_eq!(
        fs::read(bundle.join("Contents/embedded.provisionprofile"))?,
        b"opaque provisioning profile fixture\x00\xff"
    );
    fs::remove_dir_all(directory)?;
    Ok(())
}

#[test]
fn invalid_profile_signing_requests_preserve_destination() -> Result<(), Box<dyn Error>> {
    let directory =
        std::env::temp_dir().join(format!("sigil-profile-refusal-{}", std::process::id()));
    fs::create_dir_all(&directory)?;
    let marker = directory.join("keep");
    fs::write(&marker, b"existing destination")?;
    let app = app(Path::new("unused"), Path::new("unused-profile"));
    assert!(
        macos_app(&app, &[], &directory, &CodeSign::default())
            .is_err_and(|error| error.kind() == ErrorKind::Codesign)
    );
    assert!(
        windows_dir(&app, &[], &directory).is_err_and(|error| error.kind() == ErrorKind::Codesign)
    );
    assert_eq!(fs::read(&marker)?, b"existing destination");
    fs::remove_dir_all(directory)?;
    Ok(())
}

#[cfg(unix)]
#[test]
fn profile_symlink_is_rejected_before_signing() -> Result<(), Box<dyn Error>> {
    let directory = std::env::temp_dir().join(format!("sigil-profile-link-{}", std::process::id()));
    fs::create_dir_all(&directory)?;
    let original = directory.join("original");
    let profile = directory.join("profile");
    let bundle = directory.join("Fixture.app");
    fs::write(&original, b"retained profile")?;
    std::os::unix::fs::symlink(&original, &profile)?;
    let executable = std::env::current_exe()?;
    let result = macos_app(
        &app(&executable, &profile),
        &[],
        &bundle,
        &CodeSign::default().identity("sigil-profile-fixture-identity-does-not-exist"),
    );
    assert!(result.is_err_and(
        |error| error.kind() == ErrorKind::Symlink && error.path() == Some(profile.as_path())
    ));
    assert!(!bundle.join("Contents/embedded.provisionprofile").exists());
    assert_eq!(fs::read(&original)?, b"retained profile");
    fs::remove_dir_all(directory)?;
    Ok(())
}

#[test]
fn a_missing_executable_keeps_the_io_error_as_its_source() -> Result<(), Box<dyn Error>> {
    let directory = std::env::temp_dir().join(format!("sigil-io-source-{}", std::process::id()));
    let missing = directory.join("no-such-executable");
    let app = AppBundle::new("Fixture", &missing);
    let error =
        windows_dir(&app, &[], &directory.join("out")).expect_err("the executable is missing");
    assert_eq!(error.kind(), ErrorKind::Io);
    let cause = error
        .source()
        .and_then(|cause| cause.downcast_ref::<std::io::Error>());
    assert_eq!(
        cause.map(std::io::Error::kind),
        Some(std::io::ErrorKind::NotFound)
    );
    if directory.exists() {
        fs::remove_dir_all(directory)?;
    }
    Ok(())
}
