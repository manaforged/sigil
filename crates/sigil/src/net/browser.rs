use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use wasm_bindgen_futures::js_sys::Uint8Array;

use crate::{Error, ErrorKind};

pub(crate) fn fetch_bytes(url: String, done: impl FnOnce(Result<Vec<u8>, Error>) + 'static) {
    wasm_bindgen_futures::spawn_local(async move {
        let outcome: Result<Vec<u8>, Error> = async {
            let window = web_sys::window()
                .ok_or_else(|| Error::fixed(ErrorKind::Fetch, "no browser window"))?;
            let response: web_sys::Response = JsFuture::from(window.fetch_with_str(&url))
                .await
                .and_then(JsCast::dyn_into)
                .map_err(|value| js_error(&url, &value))?;
            if !response.ok() {
                return Err(Error::message(
                    ErrorKind::Http,
                    format_args!("{url} returned HTTP {}", response.status()),
                ));
            }
            let body = response
                .array_buffer()
                .map_err(|value| js_error(&url, &value))?;
            let buffer = JsFuture::from(body)
                .await
                .map_err(|value| js_error(&url, &value))?;
            Ok(Uint8Array::new(&buffer).to_vec())
        }
        .await;
        done(outcome);
    });
}

fn js_error(url: &str, value: &JsValue) -> Error {
    Error::message(ErrorKind::Fetch, format_args!("{url}: {value:?}"))
}
