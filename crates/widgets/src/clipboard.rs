//! Shared clipboard for text widgets and `sigil::clipboard`.
//!
//! The default handle is a no-op so headless tests do not touch the
//! OS pasteboard. `sigil::launch` / `sigil::run_tui` install the OS
//! clipboard before the tree builds.

use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

/// Write failed. The handle did not store the text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClipError(String);

impl ClipError {
    /// Build an error from a displayable reason.
    pub fn new(msg: impl fmt::Display) -> Self {
        Self(msg.to_string())
    }
}

impl fmt::Display for ClipError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ClipError {}

/// Platform clipboard. Text fields and the host service share this.
pub trait Clipboard {
    /// Write `text` to the clipboard.
    ///
    /// # Errors
    ///
    /// Returns when the handle cannot store the text (no-op default,
    /// or OS pasteboard failure).
    fn set(&self, text: &str) -> Result<(), ClipError>;
    /// Read the current clipboard contents.
    fn get(&self) -> Option<String>;
}

/// Discarding clipboard. Default until the host installs a real one.
pub struct NoopClipboard;

impl Clipboard for NoopClipboard {
    fn set(&self, _text: &str) -> Result<(), ClipError> {
        Err(ClipError::new("install a clipboard with set_clipboard"))
    }
    fn get(&self) -> Option<String> {
        None
    }
}

thread_local! {
    static CURRENT: RefCell<Rc<dyn Clipboard>> = RefCell::new(Rc::new(NoopClipboard));
}

/// Install the process clipboard. Call once from `launch` / `run_tui`.
pub fn set_clipboard(clip: impl Clipboard + 'static) {
    CURRENT.with(|slot| *slot.borrow_mut() = Rc::new(clip));
}

/// Clipboard the next text field uses when the builder omits `.clipboard()`.
#[must_use]
pub fn clipboard() -> Rc<dyn Clipboard> {
    CURRENT.with(|slot| Rc::clone(&*slot.borrow()))
}

/// Write through a handle. Logs when the handle cannot store the text.
pub(crate) fn write(clip: &dyn Clipboard, text: &str) -> Result<(), ClipError> {
    match clip.set(text) {
        Ok(()) => Ok(()),
        Err(err) => {
            tracing::warn!(target: "sigil::clipboard", error = %err, "clipboard write failed");
            Err(err)
        }
    }
}

#[cfg(test)]
#[path = "clipboard_tests.rs"]
mod tests;
