//! Clipboard: `copy` / `paste`, and [`set_clipboard`] for embed hosts.
//!
//! Text fields and these functions share one handle. `launch` and
//! `run_tui` install the OS pasteboard. A game calls [`set_clipboard`]
//! with its own [`Clipboard`].

use crate::{AppResult, Error};

pub use sigil_widgets::{Clipboard, set_clipboard};

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

/// Point text fields at the OS clipboard. Called from [`crate::launch`].
#[cfg(any(feature = "desktop", feature = "tui"))]
pub(crate) fn install() {
    set_clipboard(OsClipboard);
}

/// Write `text` to the installed clipboard.
///
/// # Errors
///
/// Returns when the handle cannot store the text. `launch` and
/// `run_tui` install the OS pasteboard. An embed host must call
/// [`set_clipboard`] first.
pub fn copy(text: &str) -> AppResult<()> {
    sigil_widgets::clipboard().set(text).map_err(Error::new)
}

/// Read the installed clipboard as a string.
///
/// # Errors
///
/// Returns when the pasteboard is empty, holds a non-text payload, or
/// no handle is installed.
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
#[allow(clippy::unwrap_used, clippy::panic)]
#[path = "clipboard_tests.rs"]
mod tests;
