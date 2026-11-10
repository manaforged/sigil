use std::cell::RefCell;
use std::rc::Rc;

use crate::{AppResult, Error, ErrorKind};

#[cfg(feature = "clipboard")]
mod os;
#[cfg(feature = "clipboard")]
pub use os::OsClipboard;
#[cfg(feature = "desktop")]
pub(crate) use os::install_os;

/// Whether the OS took the text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Delivery {
    /// The confirmed delivery.
    Confirmed,
    /// The requested delivery.
    Requested,
}

/// A clipboard back end.
pub trait Clipboard {
    /// Writes text to the clipboard.
    fn set(&self, text: &str) -> Result<Delivery, Error>;
    /// Returns the get.
    fn get(&self) -> Option<String>;

    /// Returns the image.
    #[cfg(feature = "clipboard-image")]
    fn image(&self) -> Result<Option<crate::kernel::ImageSource>, Error> {
        Ok(None)
    }

    /// Writes an image to the clipboard.
    #[cfg(feature = "clipboard-image")]
    fn set_image(&self, _image: &crate::kernel::ImageSource) -> Result<Delivery, Error> {
        Err(Error::fixed(
            ErrorKind::Clipboard,
            "this clipboard does not accept images",
        ))
    }
}

kittest_pub! {
    /// `Clipboard` implementation used when no clipboard is installed.
    #[doc(hidden)]
    #[derive(Debug)]
    pub struct NoopClipboard;
}

impl Clipboard for NoopClipboard {
    fn set(&self, _text: &str) -> Result<Delivery, Error> {
        Err(Error::fixed(
            ErrorKind::Clipboard,
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

/// Replace the installed clipboard.
pub fn install(clip: impl Clipboard + 'static) {
    CURRENT.with(|slot| *slot.borrow_mut() = Rc::new(clip));
}

/// Returns the installed clipboard.
#[must_use]
pub fn clipboard() -> Rc<dyn Clipboard> {
    CURRENT.with(|slot| Rc::clone(&*slot.borrow()))
}

pub(crate) fn write(clip: &dyn Clipboard, text: &str) -> Result<Delivery, Error> {
    clip.set(text).inspect_err(
        |err| tracing::warn!(target: "sigil::clipboard", error = %err, "clipboard write failed"),
    )
}

/// Write text to the installed clipboard.
pub fn copy(text: &str) -> AppResult<Delivery> {
    clipboard().set(text)
}

/// Read text from the installed clipboard.
pub fn paste() -> AppResult<String> {
    clipboard()
        .get()
        .ok_or_else(|| Error::fixed(ErrorKind::Io, "clipboard is empty"))
}

/// Reads an image from the installed clipboard.
#[cfg(feature = "clipboard-image")]
pub fn image() -> AppResult<Option<crate::kernel::ImageSource>> {
    clipboard().image()
}

/// Copies an image to the installed clipboard.
#[cfg(feature = "clipboard-image")]
pub fn copy_image(image: &crate::kernel::ImageSource) -> Result<Delivery, Error> {
    clipboard().set_image(image)
}

#[cfg(test)]
#[path = "clipboard_tests.rs"]
mod tests;
