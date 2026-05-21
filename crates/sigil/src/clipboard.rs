//! System clipboard primitive — `sigil::clipboard::copy(text)` and
//! `sigil::clipboard::paste()`.
//!
//! Wraps [`arboard::Clipboard`] behind a thread-local cache so callers do
//! not pay handle construction (Win32 `OpenClipboard`, X11 selection
//! negotiation, etc.) on every call. The handle stays open for the
//! lifetime of the thread.
//!
//! ```no_run
//! use sigil::clipboard;
//!
//! # fn demo() -> sigil::AppResult<()> {
//! clipboard::copy("Hello world")?;
//! let text = clipboard::paste()?;
//! assert_eq!(text, "Hello world");
//! # Ok(())
//! # }
//! ```
//!
//! Both calls map `arboard` errors into [`sigil::Error`](crate::Error)
//! via [`sigil::Error::new`](crate::Error::new), so they fit the same
//! `?`-chain everything else in sigil uses.

use std::cell::RefCell;

use arboard::Clipboard;

use crate::{AppResult, Error};

thread_local! {
    /// Lazily-constructed per-thread clipboard handle.
    ///
    /// Stored as `RefCell<Option<_>>` rather than `OnceCell` so a failed
    /// platform-init can be retried on a later call (e.g. an X11 server
    /// that was not yet reachable on first use). After a successful
    /// init the handle is reused for the lifetime of the thread.
    static HANDLE: RefCell<Option<Clipboard>> = const { RefCell::new(None) };
}

/// Write `text` to the system clipboard.
///
/// Returns the platform error wrapped in [`sigil::Error`](crate::Error)
/// if the clipboard handle cannot be opened or written.
pub fn copy(text: &str) -> AppResult<()> {
    with_handle(|clip| clip.set_text(text.to_owned()).map_err(Error::new))
}

/// Read the current clipboard contents as a string.
///
/// Returns the platform error wrapped in [`sigil::Error`](crate::Error)
/// when the clipboard is empty, holds a non-text payload, or the
/// handle cannot be opened.
pub fn paste() -> AppResult<String> {
    with_handle(|clip| clip.get_text().map_err(Error::new))
}

fn with_handle<R>(f: impl FnOnce(&mut Clipboard) -> AppResult<R>) -> AppResult<R> {
    HANDLE.with(|cell| {
        let mut slot = cell.borrow_mut();
        if slot.is_none() {
            *slot = Some(Clipboard::new().map_err(Error::new)?);
        }
        // Safe: branch above guarantees `Some`. `expect` for the
        // hygiene-required reason string instead of `unwrap`.
        let handle = slot
            .as_mut()
            .expect("clipboard handle was just initialised above");
        f(handle)
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    /// On Windows CI / dev rigs the system clipboard is always available;
    /// on headless Linux runners arboard returns an init error. Skip
    /// the roundtrip there rather than failing the test suite.
    #[test]
    fn copy_paste_roundtrip_when_platform_clipboard_available() {
        let probe = copy("");
        if probe.is_err() {
            eprintln!(
                "skipping clipboard roundtrip — platform clipboard unavailable: {:?}",
                probe.err()
            );
            return;
        }
        let payload = "sigil clipboard test \u{1f5d2}";
        copy(payload).expect("copy succeeds after probe");
        let read_back = paste().expect("paste succeeds after copy");
        assert_eq!(read_back, payload);
    }
}
