use std::fs::{self, File, Metadata, OpenOptions};
use std::io::{self, Write};
use std::path::Path;

pub(crate) fn create_new(path: &Path, mode: u32) -> io::Result<File> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, mode);
    #[cfg(not(unix))]
    drop(mode);
    options.open(path)
}

pub(crate) fn write_new(path: &Path, contents: &[u8], mode: u32) -> io::Result<()> {
    create_new(path, mode)?.write_all(contents)
}

pub(crate) fn replace(path: &Path, contents: &[u8]) -> io::Result<()> {
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_symlink() || !meta.is_file() => {
            return Err(io::Error::other(format!(
                "{} is not a regular file; refusing to overwrite",
                path.display()
            )));
        }
        Ok(_) => fs::remove_file(path)?,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    write_new(path, contents, 0o644)
}

pub(crate) fn copy_checked(from: &Path, checked: &Metadata, to: &Path) -> io::Result<()> {
    let mut source = File::open(from)?;
    let opened = source.metadata()?;
    if !opened.is_file() || !same_file(checked, &opened) {
        return Err(io::Error::other(format!(
            "{} changed after the symlink check",
            from.display()
        )));
    }
    let mut dest = create_new(to, 0o600)?;
    io::copy(&mut source, &mut dest)?;
    dest.set_permissions(opened.permissions())
}

#[cfg(unix)]
fn same_file(a: &Metadata, b: &Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;
    a.dev() == b.dev() && a.ino() == b.ino()
}

#[cfg(not(unix))]
fn same_file(a: &Metadata, b: &Metadata) -> bool {
    a.len() == b.len()
}
