use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use wasm_bindgen_futures::js_sys::{Promise, Uint8Array};

use crate::reactive::{Signal, signal};
use crate::tasks::Status;
use crate::text::fonts::{add_fallback_font, wake_font_listeners};
use crate::{Error, ErrorKind};

pub fn load_font_url(url: &str, face_index: u32) -> Signal<Status<(), Error>> {
    let status = signal(Status::Pending);
    let url = url.to_owned();
    wasm_bindgen_futures::spawn_local(async move {
        let outcome = match fetch_request(&url) {
            Ok(request) => match body_request(JsFuture::from(request).await) {
                Ok(body) => JsFuture::from(body)
                    .await
                    .map(|buffer| Uint8Array::new(&buffer).to_vec())
                    .map_err(|value| js_error(&value)),
                Err(error) => Err(error),
            },
            Err(error) => Err(error),
        }
        .and_then(|bytes| add_fallback_font(bytes, face_index));
        let _ = status.try_set_always(match outcome {
            Ok(()) => Status::Ok(()),
            Err(error) => Status::Err(error),
        });
        wake_font_listeners();
    });
    status
}

fn fetch_request(url: &str) -> Result<Promise, Error> {
    web_sys::window()
        .map(|window| window.fetch_with_str(url))
        .ok_or_else(|| Error::fixed(ErrorKind::Fetch, "font fetch failed: no browser window"))
}

fn body_request(fetched: Result<JsValue, JsValue>) -> Result<Promise, Error> {
    let response: web_sys::Response = fetched
        .and_then(JsCast::dyn_into)
        .map_err(|value| js_error(&value))?;
    if !response.ok() {
        return Err(Error::message(
            ErrorKind::Http,
            format_args!("font fetch returned HTTP {}", response.status()),
        ));
    }
    response.array_buffer().map_err(|value| js_error(&value))
}

fn js_error(value: &JsValue) -> Error {
    Error::message(
        ErrorKind::Fetch,
        format_args!("font fetch failed: {value:?}"),
    )
}
