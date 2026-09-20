use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
#[cfg(target_os = "macos")]
use std::process::Command;

/// A file copied into the bundle after the executable.
pub struct ResourceCopy<'a> {
    /// Source path on disk.
    pub from: &'a Path,
    /// Destination relative to `Contents/Resources` (macOS) or the app dir (Windows).
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
    /// Built executable to copy.
    pub executable: &'a Path,
    /// File name inside `Contents/MacOS` or the Windows app dir.
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
}

impl fmt::Display for PackError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing(path) => write!(f, "missing {}", path.display()),
            Self::Io(error) => write!(f, "{error}"),
            Self::Codesign(error) => write!(f, "codesign: {error}"),
        }
    }
}

impl std::error::Error for PackError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Missing(_) | Self::Codesign(_) => None,
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

fn copy_file(from: &Path, to: &Path) -> Result<(), PackError> {
    if !from.exists() {
        return Err(PackError::Missing(from.to_path_buf()));
    }
    if let Some(parent) = to.parent() {
        fs::create_dir_all(parent).map_err(PackError::Io)?;
    }
    fs::copy(from, to).map(|_| ()).map_err(PackError::Io)
}

fn copy_tree(from: &Path, to: &Path) -> Result<(), PackError> {
    if from.is_file() {
        return copy_file(from, to);
    }
    if !from.is_dir() {
        return Err(PackError::Missing(from.to_path_buf()));
    }
    fs::create_dir_all(to).map_err(PackError::Io)?;
    for entry in fs::read_dir(from).map_err(PackError::Io)? {
        let entry = entry.map_err(PackError::Io)?;
        let name = entry.file_name();
        copy_tree(&entry.path(), &to.join(name))?;
    }
    Ok(())
}

fn info_plist(app: &AppBundle<'_>) -> String {
    let name = xml(app.name);
    let ident = xml(app.identifier);
    let version = xml(app.version);
    let exe = xml(app.exe_name);
    let min = xml(app.min_system);
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
         <plist version=\"1.0\">\n<dict>\n\
         <key>CFBundleDevelopmentRegion</key><string>English</string>\n\
         <key>CFBundleDisplayName</key><string>{name}</string>\n\
         <key>CFBundleExecutable</key><string>{exe}</string>\n\
         <key>CFBundleIconFile</key><string>icon.icns</string>\n\
         <key>CFBundleIdentifier</key><string>{ident}</string>\n\
         <key>CFBundleInfoDictionaryVersion</key><string>6.0</string>\n\
         <key>CFBundleName</key><string>{name}</string>\n\
         <key>CFBundlePackageType</key><string>APPL</string>\n\
         <key>CFBundleShortVersionString</key><string>{version}</string>\n\
         <key>CFBundleVersion</key><string>{version}</string>\n\
         <key>LSMinimumSystemVersion</key><string>{min}</string>\n\
         <key>NSHighResolutionCapable</key><true/>\n\
         </dict>\n</plist>\n"
    )
}

#[cfg(target_os = "macos")]
fn adhoc_sign(app: &Path) -> Result<(), PackError> {
    let status = Command::new("/usr/bin/codesign")
        .arg("--force")
        .arg("--sign")
        .arg("-")
        .arg(app)
        .status()
        .map_err(PackError::Io)?;
    if status.success() {
        Ok(())
    } else {
        Err(PackError::Codesign(status.to_string()))
    }
}

#[cfg(not(target_os = "macos"))]
fn adhoc_sign(_app: &Path) -> Result<(), PackError> {
    Ok(())
}

/// Write a macOS `.app` at `dest` and ad-hoc sign it on macOS.
///
/// Extra [`ResourceCopy`] entries land under `Contents/Resources`.
///
/// # Errors
///
/// Returns [`PackError::Missing`] when the executable or a resource is absent,
/// [`PackError::Io`] on filesystem failure, and [`PackError::Codesign`] when
/// `codesign` fails.
pub fn macos_app(
    app: &AppBundle<'_>,
    resources: &[ResourceCopy<'_>],
    dest: &Path,
) -> Result<(), PackError> {
    if dest.exists() {
        fs::remove_dir_all(dest).map_err(PackError::Io)?;
    }
    let macos = dest.join("Contents/MacOS");
    let resources_dir = dest.join("Contents/Resources");
    fs::create_dir_all(&macos).map_err(PackError::Io)?;
    fs::create_dir_all(&resources_dir).map_err(PackError::Io)?;
    copy_file(app.executable, &macos.join(app.exe_name))?;
    if let Some(icon) = app.icon {
        copy_file(icon, &resources_dir.join("icon.icns"))?;
    }
    fs::write(dest.join("Contents/Info.plist"), info_plist(app)).map_err(PackError::Io)?;
    for copy in resources {
        copy_tree(copy.from, &resources_dir.join(copy.to))?;
    }
    adhoc_sign(dest)
}

/// Write a Windows application directory at `dest`.
///
/// The executable is copied as [`AppBundle::exe_name`]. Extra resources keep
/// their relative destinations.
///
/// # Errors
///
/// Returns [`PackError::Missing`] or [`PackError::Io`].
pub fn windows_dir(
    app: &AppBundle<'_>,
    resources: &[ResourceCopy<'_>],
    dest: &Path,
) -> Result<(), PackError> {
    if dest.exists() {
        fs::remove_dir_all(dest).map_err(PackError::Io)?;
    }
    fs::create_dir_all(dest).map_err(PackError::Io)?;
    copy_file(app.executable, &dest.join(app.exe_name))?;
    for copy in resources {
        copy_tree(copy.from, &dest.join(copy.to))?;
    }
    Ok(())
}
