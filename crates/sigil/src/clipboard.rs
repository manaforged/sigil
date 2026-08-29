use crate::{AppResult, Error};

pub use sigil_widgets::{Clipboard, install};

#[cfg(any(feature = "desktop", feature = "tui"))]
struct OsClipboard;

#[cfg(any(feature = "desktop", feature = "tui"))]
impl Clipboard for OsClipboard {
    fn set(&self, text: &str) -> Result<(), sigil_widgets::ClipError> {
        os_copy(text).map_err(sigil_widgets::ClipError::new)
    }

    fn get(&self) -> Option<String> {
        os_paste().ok()
    }
}

#[cfg(any(feature = "desktop", feature = "tui"))]
pub(crate) fn install_os() {
    install(OsClipboard);
}

pub fn copy(text: &str) -> AppResult<()> {
    sigil_widgets::clipboard().set(text).map_err(Error::new)
}

pub fn paste() -> AppResult<String> {
    sigil_widgets::clipboard()
        .get()
        .ok_or_else(|| Error::new(Empty))
}

#[cfg(any(feature = "desktop", feature = "tui"))]
fn os_copy(text: &str) -> AppResult<()> {
    with_handle(|clip| clip.set_text(text.to_owned()).map_err(Error::new))
}

#[cfg(any(feature = "desktop", feature = "tui"))]
fn os_paste() -> AppResult<String> {
    with_handle(|clip| clip.get_text().map_err(Error::new))
}

#[cfg(any(feature = "desktop", feature = "tui"))]
fn with_handle<R>(f: impl FnOnce(&mut arboard::Clipboard) -> AppResult<R>) -> AppResult<R> {
    use std::cell::RefCell;

    thread_local! {
        static HANDLE: RefCell<Option<arboard::Clipboard>> = const { RefCell::new(None) };
    }
    HANDLE.with(|cell| {
        let mut slot = cell.borrow_mut();
        if slot.is_none() {
            *slot = Some(arboard::Clipboard::new().map_err(Error::new)?);
        }
        let handle = slot
            .as_mut()
            .expect("clipboard handle was just initialised above");
        f(handle)
    })
}

#[derive(Debug)]
struct Empty;

impl std::fmt::Display for Empty {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("clipboard is empty")
    }
}

impl std::error::Error for Empty {}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::panic,
    reason = "test/instrumentation context; the invariant is proven by the caller"
)]
#[path = "clipboard_tests.rs"]
mod tests;
