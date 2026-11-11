#[cfg(target_arch = "wasm32")]
mod browser;
#[cfg(native_http)]
mod native;

#[cfg(target_arch = "wasm32")]
pub(crate) use browser::fetch_bytes;
#[cfg(native_http)]
pub(crate) use native::get;

use crate::Error;

pub(crate) fn fetch_failed(what: &str, err: &Error) -> Error {
    Error::message(err.kind(), format_args!("{what} fetch failed: {err}"))
}
