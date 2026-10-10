#[cfg(target_os = "macos")]
use std::{
    fs,
    os::unix::fs::{PermissionsExt, symlink},
    process::Command,
};

#[cfg(target_os = "macos")]
#[test]
fn standalone_signing_produces_a_valid_executable_and_refuses_links()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = std::env::temp_dir().join(format!("sigil-native-{}", std::process::id()));
    fs::create_dir_all(&directory)?;
    let executable = directory.join("executable");
    fs::copy(std::env::current_exe()?, &executable)?;
    let mut permissions = fs::metadata(&executable)?.permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(&executable, permissions)?;
    assert!(
        Command::new("/usr/bin/codesign")
            .arg("--remove-signature")
            .arg(&executable)
            .status()?
            .success()
    );
    assert!(
        !Command::new("/usr/bin/codesign")
            .arg("--verify")
            .arg(&executable)
            .status()?
            .success()
    );
    super::pack_cli(&[
        "codesign".into(),
        "--file".into(),
        executable.to_str().ok_or("path")?.into(),
    ])?;
    assert!(
        Command::new("/usr/bin/codesign")
            .args(["--verify", "--strict"])
            .arg(&executable)
            .status()?
            .success()
    );
    assert!(Command::new(&executable).arg("--list").status()?.success());
    let signed = fs::read(&executable)?;
    let link = directory.join("linked-executable");
    symlink(&executable, &link)?;
    assert!(matches!(
        super::CodeSign::default().sign(&link),
        Err(super::PackError::Symlink(_))
    ));
    assert_eq!(fs::read(&executable)?, signed);
    assert!(super::CodeSign::default().sign(&directory).is_err());
    fs::remove_dir_all(directory)?;
    Ok(())
}

#[cfg(not(target_os = "macos"))]
#[test]
fn native_signing_rejects_unsupported_platforms() {
    assert!(matches!(
        super::CodeSign::default().sign(std::path::Path::new("unused")),
        Err(super::PackError::Codesign(message)) if message == "native signing is supported only on macOS"
    ));
}
