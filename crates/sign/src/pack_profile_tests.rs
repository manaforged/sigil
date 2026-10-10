use std::error::Error;
use std::fs;
use std::path::Path;

use super::{AppBundle, CodeSign, PackError, macos_app, pack_cli, windows_dir};

fn app<'a>(executable: &'a Path, profile: &'a Path) -> AppBundle<'a> {
    AppBundle {
        name: "Profile fixture",
        identifier: "example.sigil.profile",
        version: "1.0.0",
        executable,
        exe_name: "fixture",
        icon: None,
        provisioning_profile: Some(profile),
        min_system: "10.15",
    }
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
        assert!(result.is_err_and(|error| error.starts_with("codesign:")));
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
    assert!(matches!(
        macos_app(&app, &[], &directory, &CodeSign::default()),
        Err(PackError::Codesign(_))
    ));
    assert!(matches!(
        windows_dir(&app, &[], &directory),
        Err(PackError::Codesign(_))
    ));
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
        &CodeSign {
            identity: Some("sigil-profile-fixture-identity-does-not-exist"),
            entitlements: None,
        },
    );
    assert!(matches!(result, Err(PackError::Symlink(path)) if path == profile));
    assert!(!bundle.join("Contents/embedded.provisionprofile").exists());
    assert_eq!(fs::read(&original)?, b"retained profile");
    fs::remove_dir_all(directory)?;
    Ok(())
}
