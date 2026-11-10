use crate::Error;
use crate::clipboard::{Clipboard, Delivery};
#[cfg(feature = "clipboard-image")]
use crate::kernel::ImageSource;

#[cfg(not(target_arch = "wasm32"))]
#[path = "os/native.rs"]
mod platform;
#[cfg(target_arch = "wasm32")]
#[path = "os/web.rs"]
mod platform;

/// The OS clipboard (feature `clipboard`).
#[derive(Debug)]
pub struct OsClipboard;

impl Clipboard for OsClipboard {
    fn set(&self, text: &str) -> Result<Delivery, Error> {
        platform::copy(text)
    }

    fn get(&self) -> Option<String> {
        platform::paste()
            .inspect_err(
                |err| tracing::warn!(target: "sigil::clipboard", error = %err, "clipboard read failed"),
            )
            .ok()
    }

    #[cfg(feature = "clipboard-image")]
    fn image(&self) -> Result<Option<ImageSource>, Error> {
        platform::image()
    }

    #[cfg(feature = "clipboard-image")]
    fn set_image(&self, image: &ImageSource) -> Result<Delivery, Error> {
        platform::copy_image(image)
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
