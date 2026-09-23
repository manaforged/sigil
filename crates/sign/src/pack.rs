#[cfg(target_os = "macos")]
use std::ffi::OsString;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};
#[cfg(target_os = "macos")]
use std::process::Command;

/// A file copied into the bundle after the executable.
pub struct ResourceCopy<'a> {
    /// Source path on disk. Must be a regular file or directory, not a symlink.
    pub from: &'a Path,
    /// Destination relative to `Contents/Resources` (macOS) or the app dir (Windows).
    /// Must be a relative path with no `..` components.
    pub to: &'a Path,
}

/// Identity and files for a native application bundle.
pub struct AppBundle<'a> {
    /// Display and bundle name.
    pub name: &'a str,
    /// Reverse-DNS identifier.
    pub identifier: &'a str,
    /// Short version string.
    pub version: &'a str,
    /// Built executable to copy. Must be a regular file.
    pub executable: &'a Path,
    /// File name inside `Contents/MacOS` or the Windows app dir. One path component.
    pub exe_name: &'a str,
    /// Optional `.icns` copied to `Resources/icon.icns`.
    pub icon: Option<&'a Path>,
    /// `LSMinimumSystemVersion`. Ignored on Windows.
    pub min_system: &'a str,
}

/// A bundling failure.
#[derive(Debug)]
pub enum PackError {
    /// A required path was missing.
    Missing(PathBuf),
    /// A filesystem operation failed.
    Io(io::Error),
    /// `codesign` failed on macOS.
    Codesign(String),
    /// A path would land outside the bundle.
    Escape(PathBuf),
    /// A symlink or reparse point was in the source tree.
    Symlink(PathBuf),
    /// `--out` exists and is not a previous bundle this crate wrote.
    Occupied(PathBuf),
    /// The executable is not a regular file.
    NotFile(PathBuf),
}

impl fmt::Display for PackError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing(path) => write!(f, "missing {}", path.display()),
            Self::Io(error) => write!(f, "{error}"),
            Self::Codesign(error) => write!(f, "codesign: {error}"),
            Self::Escape(path) => write!(f, "path escapes bundle: {}", path.display()),
            Self::Symlink(path) => write!(f, "symlink refused: {}", path.display()),
            Self::Occupied(path) => write!(f, "refusing to delete {}", path.display()),
            Self::NotFile(path) => write!(f, "not a regular file: {}", path.display()),
        }
    }
}

impl std::error::Error for PackError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            _ => None,
        }
    }
}

fn xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

pub(crate) fn one_component(name: &str) -> Result<&Path, PackError> {
    let path = Path::new(name);
    let mut parts = path.components();
    match (parts.next(), parts.next()) {
        (Some(Component::Normal(_)), None) => Ok(path),
        _ => Err(PackError::Escape(path.to_path_buf())),
    }
}

fn relative_inside(root: &Path, rel: &Path) -> Result<PathBuf, PackError> {
    if rel.is_absolute() {
        return Err(PackError::Escape(rel.to_path_buf()));
    }
    for part in rel.components() {
        if !matches!(part, Component::Normal(_) | Component::CurDir) {
            return Err(PackError::Escape(rel.to_path_buf()));
        }
    }
    Ok(root.join(rel))
}

fn refuse_link(path: &Path) -> Result<fs::Metadata, PackError> {
    let meta = fs::symlink_metadata(path).map_err(PackError::Io)?;
    if meta.file_type().is_symlink() {
        return Err(PackError::Symlink(path.to_path_buf()));
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const REPARSE_POINT: u32 = 0x400;
        if meta.file_attributes() & REPARSE_POINT != 0 {
            return Err(PackError::Symlink(path.to_path_buf()));
        }
    }
    Ok(meta)
}

fn copy_regular_file(from: &Path, to: &Path) -> Result<(), PackError> {
    let meta = refuse_link(from)?;
    if !meta.is_file() {
        return Err(PackError::NotFile(from.to_path_buf()));
    }
    if let Some(parent) = to.parent() {
        fs::create_dir_all(parent).map_err(PackError::Io)?;
    }
    fs::copy(from, to).map(|_| ()).map_err(PackError::Io)
}

fn copy_tree(from: &Path, to: &Path) -> Result<(), PackError> {
    let meta = refuse_link(from)?;
    if meta.is_file() {
        return copy_regular_file(from, to);
    }
    if !meta.is_dir() {
        return Err(PackError::Missing(from.to_path_buf()));
    }
    fs::create_dir_all(to).map_err(PackError::Io)?;
    for entry in fs::read_dir(from).map_err(PackError::Io)? {
        let entry = entry.map_err(PackError::Io)?;
        let name = entry.file_name();
        let text = name
            .to_str()
            .ok_or_else(|| PackError::Escape(PathBuf::from(&name)))?;
        one_component(text)?;
        copy_tree(&entry.path(), &to.join(name))?;
    }
    Ok(())
}

const PLIST_MARKER: &str = "<key>SigilPack</key><true/>";

fn previous_bundle(dest: &Path) -> bool {
    dest.join(".sigil-pack").is_file()
        || fs::read_to_string(dest.join("Contents/Info.plist"))
            .is_ok_and(|plist| plist.contains(PLIST_MARKER))
}

fn mark_bundle(dest: &Path) -> Result<(), PackError> {
    fs::write(dest.join(".sigil-pack"), b"sigil-pack\n").map_err(PackError::Io)
}

fn prepare_dest(dest: &Path) -> Result<(), PackError> {
    let meta = match fs::symlink_metadata(dest) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(PackError::Io(error)),
        Ok(meta) => meta,
    };
    if meta.file_type().is_symlink() {
        return Err(PackError::Occupied(dest.to_path_buf()));
    }
    if meta.is_dir() && previous_bundle(dest) {
        fs::remove_dir_all(dest).map_err(PackError::Io)?;
        return Ok(());
    }
    if meta.is_dir() && fs::read_dir(dest).map_err(PackError::Io)?.next().is_none() {
        return Ok(());
    }
    Err(PackError::Occupied(dest.to_path_buf()))
}

fn info_plist(app: &AppBundle<'_>, with_icon: bool) -> String {
    let name = xml(app.name);
    let ident = xml(app.identifier);
    let version = xml(app.version);
    let exe = xml(app.exe_name);
    let min = xml(app.min_system);
    let icon = if with_icon {
        "<key>CFBundleIconFile</key><string>icon.icns</string>\n"
    } else {
        ""
    };
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
         <plist version=\"1.0\">\n<dict>\n\
         <key>CFBundleDevelopmentRegion</key><string>English</string>\n\
         <key>CFBundleDisplayName</key><string>{name}</string>\n\
         <key>CFBundleExecutable</key><string>{exe}</string>\n\
         {icon}\
         <key>CFBundleIdentifier</key><string>{ident}</string>\n\
         <key>CFBundleInfoDictionaryVersion</key><string>6.0</string>\n\
         <key>CFBundleName</key><string>{name}</string>\n\
         <key>CFBundlePackageType</key><string>APPL</string>\n\
         <key>CFBundleShortVersionString</key><string>{version}</string>\n\
         <key>CFBundleVersion</key><string>{version}</string>\n\
         <key>LSMinimumSystemVersion</key><string>{min}</string>\n\
         <key>NSHighResolutionCapable</key><true/>\n\
         {PLIST_MARKER}\n\
         </dict>\n</plist>\n"
    )
}

/// How to codesign a macOS bundle.
///
/// The default is ad-hoc, which Gatekeeper does not accept for a
/// downloaded app. Set `identity` to a Developer ID to produce a
/// distributable signature.
#[derive(Debug, Default, Clone, Copy)]
pub struct CodeSign<'a> {
    /// Signing identity as `security find-identity` prints it. `None` is ad-hoc.
    pub identity: Option<&'a str>,
    /// Entitlements plist applied with an identity.
    pub entitlements: Option<&'a Path>,
}

impl CodeSign<'_> {
    #[cfg(target_os = "macos")]
    fn args(&self, app: &Path) -> Vec<OsString> {
        let mut args: Vec<OsString> = vec!["--force".into(), "--sign".into()];
        match self.identity {
            Some(identity) => {
                args.push(identity.into());
                args.push("--options".into());
                args.push("runtime".into());
                args.push("--timestamp".into());
                if let Some(entitlements) = self.entitlements {
                    args.push("--entitlements".into());
                    args.push(entitlements.into());
                }
            }
            None => args.push("-".into()),
        }
        args.push(app.into());
        args
    }
}

#[cfg(target_os = "macos")]
fn codesign(app: &Path, sign: &CodeSign<'_>) -> Result<(), PackError> {
    let status = Command::new("/usr/bin/codesign")
        .args(sign.args(app))
        .status()
        .map_err(PackError::Io)?;
    if status.success() {
        Ok(())
    } else {
        Err(PackError::Codesign(status.to_string()))
    }
}

/// Write a macOS `.app` at `dest`.
///
/// On macOS this codesigns with `sign`. The default [`CodeSign`] is ad-hoc
/// (`codesign --sign -`), which is not a Developer ID or notarized shipping
/// signature. An identity adds the hardened runtime and a secure timestamp,
/// which notarization requires.
///
/// Extra [`ResourceCopy`] entries land under `Contents/Resources`.
///
/// # Errors
///
/// Returns [`PackError`] when a path escapes `dest`, a symlink is present,
/// `dest` is occupied, or `codesign` fails.
pub fn macos_app(
    app: &AppBundle<'_>,
    resources: &[ResourceCopy<'_>],
    dest: &Path,
    sign: &CodeSign<'_>,
) -> Result<(), PackError> {
    prepare_dest(dest)?;
    let macos = dest.join("Contents/MacOS");
    let resources_dir = dest.join("Contents/Resources");
    fs::create_dir_all(&macos).map_err(PackError::Io)?;
    fs::create_dir_all(&resources_dir).map_err(PackError::Io)?;
    copy_regular_file(app.executable, &macos.join(one_component(app.exe_name)?))?;
    if let Some(icon) = app.icon {
        copy_regular_file(icon, &resources_dir.join("icon.icns"))?;
    }
    fs::write(
        dest.join("Contents/Info.plist"),
        info_plist(app, app.icon.is_some()),
    )
    .map_err(PackError::Io)?;
    for copy in resources {
        copy_tree(copy.from, &relative_inside(&resources_dir, copy.to)?)?;
    }
    #[cfg(target_os = "macos")]
    codesign(dest, sign)?;
    #[cfg(not(target_os = "macos"))]
    let _ = sign;
    Ok(())
}

/// Write a Windows application directory at `dest`.
///
/// # Errors
///
/// Returns [`PackError`] when a path escapes `dest` or a symlink is present.
pub fn windows_dir(
    app: &AppBundle<'_>,
    resources: &[ResourceCopy<'_>],
    dest: &Path,
) -> Result<(), PackError> {
    prepare_dest(dest)?;
    fs::create_dir_all(dest).map_err(PackError::Io)?;
    copy_regular_file(app.executable, &dest.join(one_component(app.exe_name)?))?;
    for copy in resources {
        copy_tree(copy.from, &relative_inside(dest, copy.to)?)?;
    }
    mark_bundle(dest)
}
