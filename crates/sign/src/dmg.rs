use std::fs;
use std::path::Path;
use std::process::Command;
use std::thread::sleep;
use std::time::Duration;

use crate::pack::{CodeSign, one_component, refuse_link};
use crate::{Error, ErrorKind};

const SPARE_BYTES: u64 = 20 * 1024 * 1024;
const DETACH_ATTEMPTS: u32 = 5;
const DETACH_WAIT: Duration = Duration::from_secs(1);

/// Inputs for a drag-to-Applications macOS disk image.
#[must_use]
#[derive(Debug, Clone, Copy)]
pub struct Dmg<'a> {
    app: &'a Path,
    volume: &'a str,
    background: Option<&'a Path>,
    icon: Option<&'a Path>,
    layout: Option<&'a Path>,
}

impl<'a> Dmg<'a> {
    /// Creates a disk image of the bundle at `app` with the volume name `volume`.
    pub const fn new(app: &'a Path, volume: &'a str) -> Self {
        Self {
            app,
            volume,
            background: None,
            icon: None,
            layout: None,
        }
    }

    /// Sets the background picture, or `None` for no picture.
    pub fn background(mut self, background: impl Into<Option<&'a Path>>) -> Self {
        self.background = background.into();
        self
    }

    /// Sets the volume icon, or `None` for the default icon.
    pub fn icon(mut self, icon: impl Into<Option<&'a Path>>) -> Self {
        self.icon = icon.into();
        self
    }

    /// Sets the Finder layout file that becomes `.DS_Store`, or `None` for the default layout.
    pub fn layout(mut self, layout: impl Into<Option<&'a Path>>) -> Self {
        self.layout = layout.into();
        self
    }
}

/// Builds a compressed disk image of an app bundle and signs the image.
pub fn macos_dmg(dmg: &Dmg<'_>, dest: &Path, sign: &CodeSign<'_>) -> Result<(), Error> {
    let plist = dmg.app.join("Contents/Info.plist");
    if !refuse_link(dmg.app)?.is_dir() || !plist.is_file() {
        return Err(Error::at(ErrorKind::Missing, &plist));
    }
    one_component(dmg.volume)?;
    let mount = Path::new("/Volumes").join(dmg.volume);
    if mount.exists() {
        return Err(Error::at(ErrorKind::Occupied, &mount));
    }
    let writable = dest.with_extension("rw.dmg");
    let result = build(dmg, &mount, &writable, dest).and_then(|()| sign.sign(dest));
    let removed = remove_if_present(&writable);
    result.and(removed)
}

/// Submits a signed file to Apple's notary service and staples the ticket.
pub fn notarize(file: &Path, profile: &str) -> Result<(), Error> {
    refuse_link(file)?;
    run(Command::new("/usr/bin/xcrun")
        .args(["notarytool", "submit"])
        .arg(file)
        .args(["--keychain-profile", profile, "--wait"]))?;
    run(Command::new("/usr/bin/xcrun")
        .args(["stapler", "staple"])
        .arg(file))
}

fn build(dmg: &Dmg<'_>, mount: &Path, writable: &Path, dest: &Path) -> Result<(), Error> {
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

fn fill(dmg: &Dmg<'_>, mount: &Path) -> Result<(), Error> {
    let name = dmg
        .app
        .file_name()
        .ok_or_else(|| Error::at(ErrorKind::Missing, dmg.app))?;
    run(Command::new("/usr/bin/ditto")
        .arg(dmg.app)
        .arg(mount.join(name)))?;
    std::os::unix::fs::symlink("/Applications", mount.join("Applications")).map_err(Error::io)?;
    if let Some(icon) = dmg.icon {
        fs::copy(icon, mount.join(".VolumeIcon.icns")).map_err(Error::io)?;
        run(Command::new("/usr/bin/xcrun")
            .args(["SetFile", "-a", "C"])
            .arg(mount))?;
    }
    if let Some(background) = dmg.background {
        let extension = background
            .extension()
            .ok_or_else(|| Error::at(ErrorKind::NotFile, background))?;
        let picture = mount.join(".background").with_extension(extension);
        fs::copy(background, picture).map_err(Error::io)?;
    }
    if let Some(layout) = dmg.layout {
        fs::copy(layout, mount.join(".DS_Store")).map_err(Error::io)?;
    }
    run(&mut Command::new("/bin/sync"))
}

fn detach(mount: &Path) -> Result<(), Error> {
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

fn tree_size(path: &Path) -> Result<u64, Error> {
    let meta = fs::symlink_metadata(path).map_err(Error::io)?;
    if !meta.is_dir() {
        return Ok(meta.len());
    }
    let mut total = 0;
    for entry in fs::read_dir(path).map_err(Error::io)? {
        total += tree_size(&entry.map_err(Error::io)?.path())?;
    }
    Ok(total)
}

fn remove_if_present(path: &Path) -> Result<(), Error> {
    match fs::remove_file(path) {
        Err(error) if error.kind() != std::io::ErrorKind::NotFound => Err(Error::io(error)),
        _ => Ok(()),
    }
}

fn run(command: &mut Command) -> Result<(), Error> {
    let tool = command.get_program().to_string_lossy().into_owned();
    let output = command.output().map_err(Error::io)?;
    if output.status.success() {
        return Ok(());
    }
    Err(Error::bare(
        ErrorKind::Tool,
        format!(
            "{tool}: {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        ),
    ))
}
