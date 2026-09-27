#[cfg(feature = "clipboard-image")]
use crate::clipboard::ClipError;
use crate::clipboard::Delivery;
#[cfg(feature = "clipboard-image")]
use crate::kernel::ImageSource;
use crate::{AppResult, Error};

#[cfg(feature = "clipboard-image")]
pub(super) fn image() -> AppResult<Option<ImageSource>> {
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

pub(super) fn copy(text: &str) -> AppResult<Delivery> {
    with_handle(|clip| clip.set_text(text.to_owned()).map_err(Error::io))?;
    Ok(Delivery::Confirmed)
}

pub(super) fn paste() -> AppResult<String> {
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
