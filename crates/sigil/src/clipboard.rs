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

    #[cfg(feature = "clipboard-image")]
    fn set_image(&self, _image: &crate::kernel::ImageSource) -> Result<Delivery, ClipError> {
        Err(ClipError::new("this clipboard does not accept images"))
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
    clip.set(text).inspect_err(
        |err| tracing::warn!(target: "sigil::clipboard", error = %err, "clipboard write failed"),
    )
}

pub fn copy(text: &str) -> AppResult<Delivery> {
    clipboard().set(text).map_err(Error::io)
}

pub fn paste() -> AppResult<String> {
    clipboard()
        .get()
        .ok_or_else(|| Error::io(ClipError::new("clipboard is empty")))
}

#[cfg(feature = "clipboard-image")]
pub fn image() -> AppResult<Option<crate::kernel::ImageSource>> {
    clipboard().image().map_err(Error::io)
}

#[cfg(feature = "clipboard-image")]
pub fn copy_image(image: &crate::kernel::ImageSource) -> Result<Delivery, ClipError> {
    clipboard().set_image(image)
}

#[cfg(test)]
#[path = "clipboard_tests.rs"]
mod tests;
