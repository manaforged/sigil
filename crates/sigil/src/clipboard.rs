use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

use crate::{AppResult, Error};

#[cfg(feature = "clipboard")]
mod os;
#[cfg(feature = "clipboard")]
pub use os::OsClipboard;
#[cfg(feature = "desktop")]
pub(crate) use os::install_os;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClipError(String);

impl ClipError {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Delivery {
    Confirmed,
    Requested,
}

pub trait Clipboard {
    fn set(&self, text: &str) -> Result<Delivery, ClipError>;
    fn get(&self) -> Option<String>;

    #[cfg(feature = "clipboard-image")]
    fn image(&self) -> Result<Option<crate::kernel::ImageSource>, ClipError> {
        Ok(None)
    }
}

pub struct NoopClipboard;

impl Clipboard for NoopClipboard {
    fn set(&self, _text: &str) -> Result<Delivery, ClipError> {
        Err(ClipError::new(
            "no clipboard handle is installed; call install first",
        ))
    }
    fn get(&self) -> Option<String> {
        None
    }
}

thread_local! {
    static CURRENT: RefCell<Rc<dyn Clipboard>> = RefCell::new(Rc::new(NoopClipboard));
}

pub fn install(clip: impl Clipboard + 'static) {
    CURRENT.with(|slot| *slot.borrow_mut() = Rc::new(clip));
}

#[must_use]
pub fn clipboard() -> Rc<dyn Clipboard> {
    CURRENT.with(|slot| Rc::clone(&*slot.borrow()))
}

pub(crate) fn write(clip: &dyn Clipboard, text: &str) -> Result<Delivery, ClipError> {
    match clip.set(text) {
        Ok(delivery) => Ok(delivery),
        Err(err) => {
            tracing::warn!(target: "sigil::clipboard", error = %err, "clipboard write failed");
            Err(err)
        }
    }
}

pub fn copy(text: &str) -> AppResult<Delivery> {
    clipboard().set(text).map_err(Error::io)
}

pub fn paste() -> AppResult<String> {
    clipboard().get().ok_or_else(|| Error::io(Empty))
}

#[cfg(feature = "clipboard-image")]
pub fn image() -> AppResult<Option<crate::kernel::ImageSource>> {
    clipboard().image().map_err(Error::io)
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
#[path = "clipboard_tests.rs"]
mod tests;
