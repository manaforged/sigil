mod entries;
pub(crate) mod layer;
#[cfg(feature = "kittest")]
pub mod mock;
#[cfg(feature = "native-dialog")]
mod native;
mod picker;

use std::convert::Infallible;
use std::path::{Path, PathBuf};

use crate::reactive::{Signal, signal};
use crate::tasks::Status;

/// Result of a file dialog.
pub type Picked<T> = Signal<Status<Option<T>, Infallible>>;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct FileFilter {
    pub(crate) name: String,
    pub(crate) extensions: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PickKind {
    File,
    Files,
    Folder,
    Save,
}

/// File picker (feature `dialog`).
#[derive(Clone, Debug, Default)]
pub struct FileDialog {
    filters: Vec<FileFilter>,
    directory: Option<PathBuf>,
    file_name: Option<String>,
}

impl FileDialog {
    /// Creates a file dialog.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the filter.
    #[must_use]
    pub fn filter(mut self, name: &str, extensions: &[&str]) -> Self {
        self.filters.push(FileFilter {
            name: name.to_owned(),
            extensions: extensions.iter().map(|ext| (*ext).to_owned()).collect(),
        });
        self
    }

    /// Sets the directory.
    #[must_use]
    pub fn directory(mut self, dir: impl AsRef<Path>) -> Self {
        self.directory = Some(dir.as_ref().to_path_buf());
        self
    }

    /// Sets the file name.
    #[must_use]
    pub fn file_name(mut self, name: &str) -> Self {
        self.file_name = Some(name.to_owned());
        self
    }

    /// Returns the open file.
    #[must_use]
    pub fn open_file(self) -> Picked<PathBuf> {
        self.pick_one(PickKind::File)
    }

    /// Returns the open files.
    #[must_use]
    pub fn open_files(self) -> Picked<Vec<PathBuf>> {
        #[cfg(feature = "kittest")]
        if let Some(response) = mock::take_paths() {
            return signal(Status::Ok(response));
        }
        #[cfg(feature = "native-dialog")]
        if !layer::in_app() {
            return native::open_files(self);
        }
        let out = signal(Status::Pending);
        layer::push_files(self, PickKind::Files, move |paths| {
            out.update(|status| *status = Status::Ok(paths));
        });
        out
    }

    /// Returns the pick folder.
    #[must_use]
    pub fn pick_folder(self) -> Picked<PathBuf> {
        self.pick_one(PickKind::Folder)
    }

    /// Returns the save file.
    #[must_use]
    pub fn save_file(self) -> Picked<PathBuf> {
        self.pick_one(PickKind::Save)
    }

    fn pick_one(self, kind: PickKind) -> Picked<PathBuf> {
        #[cfg(feature = "kittest")]
        if let Some(response) = mock::take_path() {
            return signal(Status::Ok(response));
        }
        #[cfg(feature = "native-dialog")]
        if !layer::in_app() {
            return native::pick_one(self, kind);
        }
        let out = signal(Status::Pending);
        layer::push_files(self, kind, move |paths| {
            out.update(|status| {
                *status = Status::Ok(paths.and_then(|paths| paths.into_iter().next()));
            });
        });
        out
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AlertKind {
    Message,
    Confirm,
}

/// Starts a dialog that picks one file to open.
#[must_use]
pub fn open_file() -> Picked<PathBuf> {
    FileDialog::new().open_file()
}

/// Starts a dialog that picks a path to save to.
#[must_use]
pub fn save_file() -> Picked<PathBuf> {
    FileDialog::new().save_file()
}

/// Starts a dialog that picks one folder.
#[must_use]
pub fn pick_folder() -> Picked<PathBuf> {
    FileDialog::new().pick_folder()
}

/// Shows a message dialog.
#[must_use]
pub fn message(
    title: impl Into<String>,
    body: impl Into<String>,
) -> Signal<Status<bool, Infallible>> {
    alert(AlertKind::Message, title.into(), body.into())
}

/// Shows a dialog that asks the user to confirm.
#[must_use]
pub fn confirm(
    title: impl Into<String>,
    body: impl Into<String>,
) -> Signal<Status<bool, Infallible>> {
    alert(AlertKind::Confirm, title.into(), body.into())
}

fn alert(kind: AlertKind, title: String, body: String) -> Signal<Status<bool, Infallible>> {
    #[cfg(feature = "kittest")]
    if let Some(ok) = mock::take_alert() {
        return signal(Status::Ok(ok));
    }
    #[cfg(feature = "native-dialog")]
    if !layer::in_app() {
        return native::alert(kind, title, body);
    }
    let out = signal(Status::Pending);
    layer::push_alert(kind, title, body, move |ok| {
        out.update(|status| *status = Status::Ok(ok));
    });
    out
}

#[cfg(test)]
mod tests;
