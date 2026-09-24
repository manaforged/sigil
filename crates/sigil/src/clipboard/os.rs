use crate::clipboard::{ClipError, Clipboard, Delivery};
#[cfg(feature = "clipboard-image")]
use crate::kernel::ImageSource;
use crate::{AppResult, Error};

pub struct OsClipboard;

impl Clipboard for OsClipboard {
    fn set(&self, text: &str) -> Result<Delivery, ClipError> {
        os_copy(text)
            .map(|()| Delivery::Confirmed)
            .map_err(ClipError::new)
    }

    fn get(&self) -> Option<String> {
        match os_paste() {
            Ok(text) => Some(text),
            Err(err) => {
                tracing::warn!(target: "sigil::clipboard", error = %err, "clipboard read failed");
                None
            }
        }
    }

    #[cfg(feature = "clipboard-image")]
    fn image(&self) -> Result<Option<ImageSource>, ClipError> {
        os_image().map_err(ClipError::new)
    }
}

#[cfg(feature = "desktop")]
pub(crate) fn install_os() {
    crate::clipboard::install(OsClipboard);
}

#[cfg(feature = "clipboard-image")]
fn os_image() -> AppResult<Option<ImageSource>> {
    with_handle(|clip| {
        let pixels = match clip.get_image() {
            Ok(pixels) => pixels,
            Err(arboard::Error::ContentNotAvailable) => return Ok(None),
            Err(error) => return Err(Error::io(error)),
        };
        let width = u32::try_from(pixels.width).map_err(Error::io)?;
        let height = u32::try_from(pixels.height).map_err(Error::io)?;
        ImageSource::from_rgba8(width, height, pixels.bytes.into_owned())
            .map(Some)
            .ok_or_else(|| Error::io(ClipError::new("clipboard image has invalid dimensions")))
    })
}

fn os_copy(text: &str) -> AppResult<()> {
    with_handle(|clip| clip.set_text(text.to_owned()).map_err(Error::io))
}

fn os_paste() -> AppResult<String> {
    with_handle(|clip| clip.get_text().map_err(Error::io))
}

fn with_handle<R>(f: impl FnOnce(&mut arboard::Clipboard) -> AppResult<R>) -> AppResult<R> {
    use std::cell::RefCell;

    thread_local! {
        static HANDLE: RefCell<Option<arboard::Clipboard>> = const { RefCell::new(None) };
    }
    HANDLE.with(|cell| {
        let mut slot = cell.borrow_mut();
        if slot.is_none() {
            *slot = Some(arboard::Clipboard::new().map_err(Error::io)?);
        }
        let handle = slot
            .as_mut()
            .expect("clipboard handle was just initialised above");
        f(handle)
    })
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::panic,
    reason = "test/instrumentation context; the invariant is proven by the caller"
)]
#[path = "os_tests.rs"]
mod tests;
