//! Native file dialogs.
//!
//! Thin, blocking wrappers over [`rfd`] that open the OS-native open / save /
//! folder pickers. The dialog is modal: the call blocks until the user picks
//! or cancels, then returns the chosen path(s). Call it from an event handler
//! (a button's `.activate(...)`, a menu item), not the build closure.
//!
//! ```rust,no_run
//! use sigil::dialog::FileDialog;
//!
//! if let Some(path) = FileDialog::new().filter("CSV", &["csv"]).open_file() {
//!     // load `path`
//! }
//! ```
//!
//! Native pickers run their own event loop while open, so the app's window
//! pauses for the duration. That matches how every desktop app presents a
//! file picker, and keeps the API a simple synchronous `Option<PathBuf>`.

use std::path::{Path, PathBuf};

/// Builder for a native file dialog. Configure filters and a starting
/// directory, then call one of the blocking pickers.
#[derive(Default)]
pub struct FileDialog {
    inner: rfd::FileDialog,
}

impl FileDialog {
    /// Start a new file dialog with no filters.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a named extension filter, e.g. `.filter("Images", &["png", "jpg"])`.
    /// Extensions omit the leading dot.
    #[must_use]
    pub fn filter(mut self, name: &str, extensions: &[&str]) -> Self {
        self.inner = self.inner.add_filter(name, extensions);
        self
    }

    /// Set the directory the dialog opens in.
    #[must_use]
    pub fn directory(mut self, dir: impl AsRef<Path>) -> Self {
        self.inner = self.inner.set_directory(dir);
        self
    }

    /// Pre-fill the file name (used by [`save_file`](Self::save_file)).
    #[must_use]
    pub fn file_name(mut self, name: &str) -> Self {
        self.inner = self.inner.set_file_name(name);
        self
    }

    /// Open a picker for a single existing file. `None` if cancelled.
    #[must_use]
    pub fn open_file(self) -> Option<PathBuf> {
        #[cfg(any(test, feature = "test-support"))]
        if let Some(response) = mock::take_path() {
            return response;
        }
        self.inner.pick_file()
    }

    /// Open a picker for one or more existing files. `None` if cancelled.
    #[must_use]
    pub fn open_files(self) -> Option<Vec<PathBuf>> {
        #[cfg(any(test, feature = "test-support"))]
        if let Some(response) = mock::take_paths() {
            return response;
        }
        self.inner.pick_files()
    }

    /// Open a folder picker. `None` if cancelled.
    #[must_use]
    pub fn pick_folder(self) -> Option<PathBuf> {
        #[cfg(any(test, feature = "test-support"))]
        if let Some(response) = mock::take_path() {
            return response;
        }
        self.inner.pick_folder()
    }

    /// Open a save-as picker, returning the chosen destination path (the
    /// file is not created). `None` if cancelled.
    #[must_use]
    pub fn save_file(self) -> Option<PathBuf> {
        #[cfg(any(test, feature = "test-support"))]
        if let Some(response) = mock::take_path() {
            return response;
        }
        self.inner.save_file()
    }
}

/// Test seam for file dialogs.
///
/// Enable the `test-support` feature (in your app's dev-dependencies) and
/// queue responses before the code under test runs; the blocking pickers
/// then return them instead of opening a native dialog, so a "user picks a
/// file, app loads it" flow is testable with no display.
///
/// ```rust,no_run
/// # #[cfg(feature = "test-support")] {
/// use sigil::dialog::{self, mock};
/// use std::path::PathBuf;
///
/// mock::push_path(Some(PathBuf::from("/tmp/data.csv")));
/// assert_eq!(dialog::open_file(), Some(PathBuf::from("/tmp/data.csv")));
/// # }
/// ```
///
/// Single-path pickers (`open_file` / `save_file` / `pick_folder`) draw from
/// [`push_path`]; the multi-file `open_files` draws from [`push_paths`].
/// Responses are consumed in call order; an empty queue falls through to the
/// real native dialog.
#[cfg(any(test, feature = "test-support"))]
pub mod mock {
    use std::cell::RefCell;
    use std::collections::VecDeque;
    use std::path::PathBuf;

    thread_local! {
        static PATHS: RefCell<VecDeque<Option<PathBuf>>> = const { RefCell::new(VecDeque::new()) };
        static PATH_LISTS: RefCell<VecDeque<Option<Vec<PathBuf>>>> =
            const { RefCell::new(VecDeque::new()) };
    }

    /// Queue the next single-path result (`open_file` / `save_file` /
    /// `pick_folder`).
    pub fn push_path(response: Option<PathBuf>) {
        PATHS.with(|queue| queue.borrow_mut().push_back(response));
    }

    /// Queue the next `open_files` (multi-file) result.
    pub fn push_paths(response: Option<Vec<PathBuf>>) {
        PATH_LISTS.with(|queue| queue.borrow_mut().push_back(response));
    }

    /// Clear all queued responses.
    pub fn reset() {
        PATHS.with(|queue| queue.borrow_mut().clear());
        PATH_LISTS.with(|queue| queue.borrow_mut().clear());
    }

    // The nesting is meaningful: outer = "was a response queued?", inner =
    // "the picker result (a path, or `None` for cancelled)".
    #[allow(clippy::option_option)]
    pub(super) fn take_path() -> Option<Option<PathBuf>> {
        PATHS.with(|queue| queue.borrow_mut().pop_front())
    }

    #[allow(clippy::option_option)]
    pub(super) fn take_paths() -> Option<Option<Vec<PathBuf>>> {
        PATH_LISTS.with(|queue| queue.borrow_mut().pop_front())
    }
}

/// Open the native single-file open picker with no filters.
#[must_use]
pub fn open_file() -> Option<PathBuf> {
    FileDialog::new().open_file()
}

/// Open the native save-as picker with no filters.
#[must_use]
pub fn save_file() -> Option<PathBuf> {
    FileDialog::new().save_file()
}

/// Open the native folder picker.
#[must_use]
pub fn pick_folder() -> Option<PathBuf> {
    FileDialog::new().pick_folder()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_chains_without_opening_a_dialog() {
        // Constructing + configuring must not open UI; only the pick_* calls
        // do. This exercises the whole builder surface headlessly.
        let _dialog = FileDialog::new()
            .filter("CSV", &["csv"])
            .filter("All", &["*"])
            .directory("/tmp")
            .file_name("export.csv");
    }

    #[test]
    fn mock_responses_bypass_the_native_picker() {
        mock::reset();
        // A single-path picker returns the queued response, no native UI.
        mock::push_path(Some(PathBuf::from("/tmp/data.csv")));
        assert_eq!(open_file(), Some(PathBuf::from("/tmp/data.csv")));
        // Responses are consumed in call order across the single-path pickers.
        mock::push_path(None);
        assert_eq!(save_file(), None);
        // The multi-file queue is independent.
        mock::push_paths(Some(vec![PathBuf::from("/a"), PathBuf::from("/b")]));
        assert_eq!(
            FileDialog::new().open_files(),
            Some(vec![PathBuf::from("/a"), PathBuf::from("/b")])
        );
        mock::reset();
    }
}
