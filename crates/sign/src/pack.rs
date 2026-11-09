#[cfg(target_os = "macos")]
use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};
#[cfg(target_os = "macos")]
use std::process::Command;

use crate::files::{copy_checked, write_new};
use crate::{Error, ErrorKind};

/// One file or directory to copy into the bundle resources.
#[derive(Debug, Clone, Copy)]
pub struct ResourceCopy<'a> {
    from: &'a Path,
    to: &'a Path,
}

impl<'a> ResourceCopy<'a> {
    /// Creates a copy of `from` to the path `to` inside the resources directory.
    #[must_use]
    pub const fn new(from: &'a Path, to: &'a Path) -> Self {
        Self { from, to }
    }
}

const MIN_SYSTEM: &str = "10.15";

/// Inputs for a macOS app bundle or a Windows app directory.
#[must_use]
#[derive(Debug, Clone, Copy)]
pub struct AppBundle<'a> {
    name: &'a str,
    identifier: &'a str,
    version: &'a str,
    executable: &'a Path,
    exe_name: &'a str,
    icon: Option<&'a Path>,
    provisioning_profile: Option<&'a Path>,
    min_system: &'a str,
}

impl<'a> AppBundle<'a> {
    /// Creates a bundle named `name` around `executable`, with the executable file also named `name`.
    pub const fn new(name: &'a str, executable: &'a Path) -> Self {
        Self {
            name,
            identifier: "",
            version: "",
            executable,
            exe_name: name,
            icon: None,
            provisioning_profile: None,
            min_system: MIN_SYSTEM,
        }
    }

    /// Sets the bundle identifier.
    pub const fn identifier(mut self, identifier: &'a str) -> Self {
        self.identifier = identifier;
        self
    }

    /// Sets the version.
    pub const fn version(mut self, version: &'a str) -> Self {
        self.version = version;
        self
    }

    /// Sets the file name of the executable inside the bundle.
    pub const fn exe_name(mut self, exe_name: &'a str) -> Self {
        self.exe_name = exe_name;
        self
    }

    /// Sets the icon file, or `None` for no icon.
    pub fn icon(mut self, icon: impl Into<Option<&'a Path>>) -> Self {
        self.icon = icon.into();
        self
    }

    /// Sets the provisioning profile to embed, or `None` for no profile.
    pub fn provisioning_profile(mut self, profile: impl Into<Option<&'a Path>>) -> Self {
        self.provisioning_profile = profile.into();
        self
    }

    /// Sets the minimum macOS version.
    pub const fn min_system(mut self, min_system: &'a str) -> Self {
        self.min_system = min_system;
        self
    }
}

fn xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

pub(crate) fn one_component(name: &str) -> Result<&Path, Error> {
    let path = Path::new(name);
    let mut parts = path.components();
    match (parts.next(), parts.next()) {
        (Some(Component::Normal(_)), None) => Ok(path),
        _ => Err(Error::at(ErrorKind::Escape, path)),
    }
}

fn relative_inside(root: &Path, rel: &Path) -> Result<PathBuf, Error> {
    let inside = !rel.is_absolute()
        && rel
            .components()
            .all(|part| matches!(part, Component::Normal(_) | Component::CurDir));
    if inside {
        Ok(root.join(rel))
    } else {
        Err(Error::at(ErrorKind::Escape, rel))
    }
}

pub(crate) fn refuse_link(path: &Path) -> Result<fs::Metadata, Error> {
    let meta = fs::symlink_metadata(path).map_err(Error::io)?;
    if meta.file_type().is_symlink() {
        return Err(Error::at(ErrorKind::Symlink, path));
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const REPARSE_POINT: u32 = 0x400;
        if meta.file_attributes() & REPARSE_POINT != 0 {
            return Err(Error::at(ErrorKind::Symlink, path));
        }
    }
    Ok(meta)
}

fn copy_regular_file(from: &Path, to: &Path) -> Result<(), Error> {
    let meta = refuse_link(from)?;
    if !meta.is_file() {
        return Err(Error::at(ErrorKind::NotFile, from));
    }
    if let Some(parent) = to.parent() {
        fs::create_dir_all(parent).map_err(Error::io)?;
    }
    copy_checked(from, &meta, to).map_err(Error::io)
}

fn copy_tree(from: &Path, to: &Path) -> Result<(), Error> {
    let meta = refuse_link(from)?;
    if meta.is_file() {
        return copy_regular_file(from, to);
    }
    if !meta.is_dir() {
        return Err(Error::at(ErrorKind::Missing, from));
    }
    fs::create_dir_all(to).map_err(Error::io)?;
    for entry in fs::read_dir(from).map_err(Error::io)? {
        let entry = entry.map_err(Error::io)?;
        let name = entry.file_name();
        let text = name
            .to_str()
            .ok_or_else(|| Error::at(ErrorKind::Escape, Path::new(&name)))?;
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

fn prepare_dest(dest: &Path) -> Result<(), Error> {
    let meta = match fs::symlink_metadata(dest) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(Error::io(error)),
        Ok(meta) => meta,
    };
    if meta.file_type().is_symlink() {
        return Err(Error::at(ErrorKind::Occupied, dest));
    }
    if meta.is_dir() && previous_bundle(dest) {
        return fs::remove_dir_all(dest).map_err(Error::io);
    }
    if meta.is_dir() && fs::read_dir(dest).map_err(Error::io)?.next().is_none() {
        return Ok(());
    }
    Err(Error::at(ErrorKind::Occupied, dest))
}

fn info_plist(app: &AppBundle<'_>) -> String {
    let name = xml(app.name);
    let ident = xml(app.identifier);
    let version = xml(app.version);
    let exe = xml(app.exe_name);
    let min = xml(app.min_system);
    let icon = if app.icon.is_some() {
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

/// Signing identity and entitlements for `codesign`; the default signs ad hoc.
#[must_use]
#[derive(Debug, Default, Clone, Copy)]
pub struct CodeSign<'a> {
    identity: Option<&'a str>,
    entitlements: Option<&'a Path>,
}

impl<'a> CodeSign<'a> {
    /// Sets the signing identity, or `None` for an ad-hoc signature.
    pub fn identity(mut self, identity: impl Into<Option<&'a str>>) -> Self {
        self.identity = identity.into();
        self
    }

    /// Sets the entitlements file, or `None` for no entitlements.
    pub fn entitlements(mut self, entitlements: impl Into<Option<&'a Path>>) -> Self {
        self.entitlements = entitlements.into();
        self
    }

    /// Signs an executable or an `.app` bundle in place.
    #[cfg(target_os = "macos")]
    pub fn sign(&self, path: &Path) -> Result<(), Error> {
        let metadata = refuse_link(path)?;
        if !(metadata.is_file()
            || metadata.is_dir() && path.extension().is_some_and(|extension| extension == "app"))
        {
            return Err(Error::text(
                ErrorKind::Codesign,
                format!(
                    "{} is not a regular executable or an .app bundle",
                    path.display()
                ),
            ));
        }
        let status = Command::new("/usr/bin/codesign")
            .args(self.args(path))
            .status()
            .map_err(Error::io)?;
        status
            .success()
            .then_some(())
            .ok_or_else(|| Error::text(ErrorKind::Codesign, status.to_string()))
    }

    /// Signs an executable or an `.app` bundle in place.
    #[cfg(not(target_os = "macos"))]
    pub fn sign(&self, path: &Path) -> Result<(), Error> {
        Err(Error::text(
            ErrorKind::Codesign,
            format!(
                "native signing of {} is supported only on macOS",
                path.display()
            ),
        ))
    }

    #[cfg(target_os = "macos")]
    fn args(&self, app: &Path) -> Vec<OsString> {
        let mut args: Vec<OsString> = vec!["--force".into(), "--sign".into()];
        match self.identity {
            Some(identity) => {
                args.extend([identity, "--options", "runtime", "--timestamp"].map(OsString::from));
                if let Some(entitlements) = self.entitlements {
                    args.extend(["--entitlements".into(), entitlements.into()]);
                }
            }
            None => args.push("-".into()),
        }
        args.push(app.into());
        args
    }
}

/// Builds a macOS `.app` bundle at `dest` and signs it.
pub fn macos_app(
    app: &AppBundle<'_>,
    resources: &[ResourceCopy<'_>],
    dest: &Path,
    sign: &CodeSign<'_>,
) -> Result<(), Error> {
    if app.provisioning_profile.is_some()
        && !sign
            .identity
            .is_some_and(|identity| !identity.trim().is_empty() && identity.trim() != "-")
    {
        return Err(Error::text(
            ErrorKind::Codesign,
            "a provisioning profile requires a signing identity",
        ));
    }
    prepare_dest(dest)?;
    let macos = dest.join("Contents/MacOS");
    let resources_dir = dest.join("Contents/Resources");
    fs::create_dir_all(&macos).map_err(Error::io)?;
    fs::create_dir_all(&resources_dir).map_err(Error::io)?;
    copy_regular_file(app.executable, &macos.join(one_component(app.exe_name)?))?;
    if let Some(icon) = app.icon {
        copy_regular_file(icon, &resources_dir.join("icon.icns"))?;
    }
    write_new(
        &dest.join("Contents/Info.plist"),
        info_plist(app).as_bytes(),
        0o644,
    )
    .map_err(Error::io)?;
    for copy in resources {
        copy_tree(copy.from, &relative_inside(&resources_dir, copy.to)?)?;
    }
    if let Some(profile) = app.provisioning_profile {
        copy_regular_file(profile, &dest.join("Contents/embedded.provisionprofile"))?;
    }
    #[cfg(target_os = "macos")]
    sign.sign(dest)?;
    #[cfg(not(target_os = "macos"))]
    let _ = sign;
    Ok(())
}

/// Builds a Windows app directory at `dest`.
pub fn windows_dir(
    app: &AppBundle<'_>,
    resources: &[ResourceCopy<'_>],
    dest: &Path,
) -> Result<(), Error> {
    if app.provisioning_profile.is_some() {
        return Err(Error::text(
            ErrorKind::Codesign,
            "provisioning profiles are supported only for macOS app bundles",
        ));
    }
    prepare_dest(dest)?;
    fs::create_dir_all(dest).map_err(Error::io)?;
    copy_regular_file(app.executable, &dest.join(one_component(app.exe_name)?))?;
    for copy in resources {
        copy_tree(copy.from, &relative_inside(dest, copy.to)?)?;
    }
    write_new(&dest.join(".sigil-pack"), b"sigil-pack\n", 0o644).map_err(Error::io)
}
