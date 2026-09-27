use crate::clipboard::{ClipError, Delivery};
#[cfg(feature = "clipboard-image")]
use crate::kernel::ImageSource;
use crate::{AppResult, Error};

pub(super) fn copy(text: &str) -> AppResult<Delivery> {
    let window = web_sys::window().ok_or_else(|| Error::io(ClipError::new("no browser window")))?;
    let pending = window.navigator().clipboard().write_text(text);
    wasm_bindgen_futures::spawn_local(async move {
        if let Err(err) = wasm_bindgen_futures::JsFuture::from(pending).await {
            tracing::warn!(target: "sigil::clipboard", error = ?err, "browser clipboard write failed");
        }
    });
    Ok(Delivery::Requested)
}

pub(super) fn paste() -> AppResult<String> {
    Err(Error::io(ClipError::new(
        "the browser clipboard has no synchronous read",
    )))
}

#[cfg(feature = "clipboard-image")]
pub(super) const fn image() -> AppResult<Option<ImageSource>> {
    Ok(None)
}
