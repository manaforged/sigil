use std::fs;
use std::path::Path;
use std::process::Command;
use std::thread::sleep;
use std::time::Duration;

use crate::pack::{CodeSign, PackError, one_component, refuse_link};

const SPARE_BYTES: u64 = 20 * 1024 * 1024;
const DETACH_ATTEMPTS: u32 = 5;
const DETACH_WAIT: Duration = Duration::from_secs(1);

/// Inputs for a drag-to-Applications macOS disk image.
pub struct Dmg<'a> {
    pub app: &'a Path,
    pub volume: &'a str,
    pub background: Option<&'a Path>,
    pub icon: Option<&'a Path>,
    pub layout: Option<&'a Path>,
}

/// Builds a compressed disk image of an app bundle and signs the image.
pub fn macos_dmg(dmg: &Dmg<'_>, dest: &Path, sign: &CodeSign<'_>) -> Result<(), PackError> {
    let plist = dmg.app.join("Contents/Info.plist");
    if !refuse_link(dmg.app)?.is_dir() || !plist.is_file() {
        return Err(PackError::Missing(plist));
    }
    one_component(dmg.volume)?;
    let mount = Path::new("/Volumes").join(dmg.volume);
    if mount.exists() {
        return Err(PackError::Occupied(mount));
    }
    let writable = dest.with_extension("rw.dmg");
    let result = build(dmg, &mount, &writable, dest).and_then(|()| sign.sign(dest));
    let removed = remove_if_present(&writable);
    result.and(removed)
}

/// Submits a signed file to Apple's notary service and staples the ticket.
pub fn notarize(file: &Path, profile: &str) -> Result<(), PackError> {
    refuse_link(file)?;
    run(Command::new("/usr/bin/xcrun")
        .args(["notarytool", "submit"])
        .arg(file)
        .args(["--keychain-profile", profile, "--wait"]))?;
    run(Command::new("/usr/bin/xcrun")
        .args(["stapler", "staple"])
        .arg(file))
}

fn build(dmg: &Dmg<'_>, mount: &Path, writable: &Path, dest: &Path) -> Result<(), PackError> {
    remove_if_present(writable)?;
    let megabytes = (tree_size(dmg.app)? + SPARE_BYTES).div_ceil(1024 * 1024);
    run(Command::new("/usr/bin/hdiutil")
        .args(["create", "-quiet", "-fs", "HFS+", "-volname", dmg.volume])
        .args(["-size", &format!("{megabytes}m")])
        .arg(writable))?;
    run(Command::new("/usr/bin/hdiutil")
        .args(["attach", "-quiet", "-noautoopen", "-noverify", "-nobrowse"])
        .arg(writable))?;
    let filled = fill(dmg, mount);
    let detached = detach(mount);
    filled.and(detached)?;
    run(Command::new("/usr/bin/hdiutil")
        .args(["convert", "-quiet", "-ov", "-format", "UDZO"])
        .args(["-imagekey", "zlib-level=9", "-o"])
        .arg(dest)
        .arg(writable))
}

fn fill(dmg: &Dmg<'_>, mount: &Path) -> Result<(), PackError> {
    let name = dmg
        .app
        .file_name()
        .ok_or_else(|| PackError::Missing(dmg.app.to_path_buf()))?;
    run(Command::new("/usr/bin/ditto")
        .arg(dmg.app)
        .arg(mount.join(name)))?;
    std::os::unix::fs::symlink("/Applications", mount.join("Applications"))
        .map_err(PackError::Io)?;
    if let Some(icon) = dmg.icon {
        fs::copy(icon, mount.join(".VolumeIcon.icns")).map_err(PackError::Io)?;
        run(Command::new("/usr/bin/xcrun")
            .args(["SetFile", "-a", "C"])
            .arg(mount))?;
    }
    if let Some(background) = dmg.background {
        let extension = background
            .extension()
            .ok_or_else(|| PackError::NotFile(background.to_path_buf()))?;
        let picture = mount.join(".background").with_extension(extension);
        fs::copy(background, picture).map_err(PackError::Io)?;
    }
    if let Some(layout) = dmg.layout {
        fs::copy(layout, mount.join(".DS_Store")).map_err(PackError::Io)?;
    }
    run(&mut Command::new("/bin/sync"))
}

fn detach(mount: &Path) -> Result<(), PackError> {
    let mut last = Ok(());
    for attempt in 1..=DETACH_ATTEMPTS {
        last = run(Command::new("/usr/bin/hdiutil")
            .args(["detach", "-quiet"])
            .arg(mount));
        if last.is_ok() {
            return last;
        }
        if attempt < DETACH_ATTEMPTS {
            sleep(DETACH_WAIT);
        }
    }
    last
}

fn tree_size(path: &Path) -> Result<u64, PackError> {
    let meta = fs::symlink_metadata(path).map_err(PackError::Io)?;
    if !meta.is_dir() {
        return Ok(meta.len());
    }
    let mut total = 0;
    for entry in fs::read_dir(path).map_err(PackError::Io)? {
        total += tree_size(&entry.map_err(PackError::Io)?.path())?;
    }
    Ok(total)
}

fn remove_if_present(path: &Path) -> Result<(), PackError> {
    match fs::remove_file(path) {
        Err(error) if error.kind() != std::io::ErrorKind::NotFound => Err(PackError::Io(error)),
        _ => Ok(()),
    }
}

fn run(command: &mut Command) -> Result<(), PackError> {
    let tool = command.get_program().to_string_lossy().into_owned();
    let output = command.output().map_err(PackError::Io)?;
    if output.status.success() {
        return Ok(());
    }
    Err(PackError::Tool(format!(
        "{tool}: {}: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr).trim()
    )))
}
