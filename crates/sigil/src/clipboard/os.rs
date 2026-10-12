use crate::clipboard::{ClipError, Clipboard, Delivery};
#[cfg(feature = "clipboard-image")]
use crate::kernel::ImageSource;

#[cfg(not(target_arch = "wasm32"))]
#[path = "os/native.rs"]
mod platform;
#[cfg(target_arch = "wasm32")]
#[path = "os/web.rs"]
mod platform;

pub struct OsClipboard;

impl Clipboard for OsClipboard {
    fn set(&self, text: &str) -> Result<Delivery, ClipError> {
        platform::copy(text).map_err(ClipError::new)
    }

    fn get(&self) -> Option<String> {
        platform::paste()
            .inspect_err(
                |err| tracing::warn!(target: "sigil::clipboard", error = %err, "clipboard read failed"),
            )
            .ok()
    }

    #[cfg(feature = "clipboard-image")]
    fn image(&self) -> Result<Option<ImageSource>, ClipError> {
        platform::image().map_err(ClipError::new)
    }

    #[cfg(feature = "clipboard-image")]
    fn set_image(&self, image: &ImageSource) -> Result<Delivery, ClipError> {
        platform::copy_image(image).map_err(ClipError::new)
    }
}

#[cfg(feature = "desktop")]
pub(crate) fn install_os() {
    crate::clipboard::install(OsClipboard);
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::panic,
    reason = "test/instrumentation context; the invariant is proven by the caller"
)]
#[path = "os_tests.rs"]
mod tests;
