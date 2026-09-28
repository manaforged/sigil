use std::time::Duration;

use tokio::sync::oneshot;
use wasm_bindgen::JsValue;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = setTimeout)]
    fn set_timeout(handler: &JsValue, timeout: i32) -> JsValue;
}

pub(crate) fn browser_timer(duration: Duration) -> oneshot::Receiver<()> {
    let (tx, rx) = oneshot::channel();
    let handler = Closure::once_into_js(move || {
        let _ = tx.send(());
    });
    let millis = i32::try_from(duration.as_millis()).unwrap_or(i32::MAX);
    drop(set_timeout(&handler, millis));
    rx
}
