use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use crate::renderer::{DeviceParts, OnLost, RendererError, WgpuRenderer};

type Slot = Rc<RefCell<Option<Result<DeviceParts, RendererError>>>>;

pub(super) struct DeviceRequest {
    slot: Slot,
    wake: OnLost,
    waiting: bool,
}

impl DeviceRequest {
    pub(super) fn new(wake: impl Fn() + Send + Sync + 'static) -> Self {
        Self {
            slot: Slot::default(),
            wake: Arc::new(wake),
            waiting: false,
        }
    }

    pub(super) fn start(&mut self, renderer: &WgpuRenderer) {
        let out = Rc::clone(&self.slot);
        let wake = Arc::clone(&self.wake);
        let request = renderer.request_device(Arc::clone(&self.wake));
        wasm_bindgen_futures::spawn_local(async move {
            *out.borrow_mut() = Some(request.await);
            wake();
        });
        self.waiting = true;
    }

    pub(super) const fn waiting(&self) -> bool {
        self.waiting
    }

    pub(super) fn take(&mut self) -> Option<Result<DeviceParts, RendererError>> {
        let result = self.slot.borrow_mut().take()?;
        self.waiting = false;
        Some(result)
    }
}

pub(super) fn report(error: &crate::app::error::Error) {
    tracing::error!("sigil-app: {error}");
    web_sys::console::error_1(&format!("sigil: {error}").into());
}
