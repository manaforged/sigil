use std::path::{Path, PathBuf};

#[derive(Default)]
pub struct FileDialog {
    inner: rfd::FileDialog,
}

impl FileDialog {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn filter(mut self, name: &str, extensions: &[&str]) -> Self {
        self.inner = self.inner.add_filter(name, extensions);
        self
    }

    #[must_use]
    pub fn directory(mut self, dir: impl AsRef<Path>) -> Self {
        self.inner = self.inner.set_directory(dir);
        self
    }

    #[must_use]
    pub fn file_name(mut self, name: &str) -> Self {
        self.inner = self.inner.set_file_name(name);
        self
    }

    #[must_use]
    pub fn open_file(self) -> Option<PathBuf> {
        #[cfg(any(test, feature = "kittest"))]
        if let Some(response) = mock::take_path() {
            return response;
        }
        self.inner.pick_file()
    }

    #[must_use]
    pub fn open_files(self) -> Option<Vec<PathBuf>> {
        #[cfg(any(test, feature = "kittest"))]
        if let Some(response) = mock::take_paths() {
            return response;
        }
        self.inner.pick_files()
    }

    #[must_use]
    pub fn pick_folder(self) -> Option<PathBuf> {
        #[cfg(any(test, feature = "kittest"))]
        if let Some(response) = mock::take_path() {
            return response;
        }
        self.inner.pick_folder()
    }

    #[must_use]
    pub fn save_file(self) -> Option<PathBuf> {
        #[cfg(any(test, feature = "kittest"))]
        if let Some(response) = mock::take_path() {
            return response;
        }
        self.inner.save_file()
    }
}

#[cfg(any(test, feature = "kittest"))]
pub mod mock {
    use std::cell::RefCell;
    use std::collections::VecDeque;
    use std::path::PathBuf;

    thread_local! {
        static PATHS: RefCell<VecDeque<Option<PathBuf>>> = const { RefCell::new(VecDeque::new()) };
        static PATH_LISTS: RefCell<VecDeque<Option<Vec<PathBuf>>>> =
            const { RefCell::new(VecDeque::new()) };
        static ALERTS: RefCell<VecDeque<bool>> = const { RefCell::new(VecDeque::new()) };
    }

    pub fn push_path(response: Option<PathBuf>) {
        PATHS.with(|queue| queue.borrow_mut().push_back(response));
    }

    pub fn push_paths(response: Option<Vec<PathBuf>>) {
        PATH_LISTS.with(|queue| queue.borrow_mut().push_back(response));
    }

    pub fn push_alert(ok: bool) {
        ALERTS.with(|queue| queue.borrow_mut().push_back(ok));
    }

    pub fn reset() {
        PATHS.with(|queue| queue.borrow_mut().clear());
        PATH_LISTS.with(|queue| queue.borrow_mut().clear());
        ALERTS.with(|queue| queue.borrow_mut().clear());
    }

    pub(super) fn take_alert() -> Option<bool> {
        ALERTS.with(|queue| queue.borrow_mut().pop_front())
    }

    #[allow(
        clippy::option_option,
        reason = "the option-of-option mirrors the three-state input the caller passes"
    )]
    pub(super) fn take_path() -> Option<Option<PathBuf>> {
        PATHS.with(|queue| queue.borrow_mut().pop_front())
    }

    #[allow(
        clippy::option_option,
        reason = "the option-of-option mirrors the three-state input the caller passes"
    )]
    pub(super) fn take_paths() -> Option<Option<Vec<PathBuf>>> {
        PATH_LISTS.with(|queue| queue.borrow_mut().pop_front())
    }
}

#[must_use]
pub fn open_file() -> Option<PathBuf> {
    FileDialog::new().open_file()
}

#[must_use]
pub fn save_file() -> Option<PathBuf> {
    FileDialog::new().save_file()
}

#[must_use]
pub fn pick_folder() -> Option<PathBuf> {
    FileDialog::new().pick_folder()
}

#[must_use]
pub fn message(title: impl Into<String>, body: impl Into<String>) -> bool {
    #[cfg(any(test, feature = "kittest"))]
    if let Some(ok) = mock::take_alert() {
        return ok;
    }
    matches!(
        rfd::MessageDialog::new()
            .set_title(title)
            .set_description(body)
            .set_buttons(rfd::MessageButtons::Ok)
            .show(),
        rfd::MessageDialogResult::Ok
    )
}

#[must_use]
pub fn confirm(title: impl Into<String>, body: impl Into<String>) -> bool {
    #[cfg(any(test, feature = "kittest"))]
    if let Some(ok) = mock::take_alert() {
        return ok;
    }
    matches!(
        rfd::MessageDialog::new()
            .set_title(title)
            .set_description(body)
            .set_buttons(rfd::MessageButtons::OkCancel)
            .show(),
        rfd::MessageDialogResult::Ok | rfd::MessageDialogResult::Yes
    )
}

#[cfg(test)]
#[path = "dialog_tests.rs"]
mod tests;
