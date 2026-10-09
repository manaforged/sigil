use wasm_bindgen::JsCast as _;
use web_sys::HtmlCanvasElement;
use winit::platform::web::{WindowAttributesExtWebSys as _, WindowExtWebSys as _};
use winit::window::{Window, WindowAttributes};

use crate::hal::Shell as _;

use super::WinitShell;
use super::web_ime::WebIme;

impl WinitShell {
    pub(crate) fn set_canvas(&mut self, id: Option<String>) {
        self.canvas = id;
    }

    pub(super) fn web_attrs(&mut self, attrs: WindowAttributes) -> WindowAttributes {
        let Some(id) = self.canvas.take() else {
            return attrs.with_append(true);
        };
        let Some(canvas) = find_canvas(&id) else {
            tracing::warn!(target: "sigil::shell", %id, "no <canvas> with this id; appending one to <body>");
            return attrs.with_append(true);
        };
        let mut attrs = attrs.with_canvas(Some(canvas));
        attrs.inner_size = None;
        attrs
    }

    pub(crate) fn set_tab_edges(&self, edges: crate::events::focus::TabEdges) {
        if let Some(ime) = &self.web_ime {
            ime.set_tab_edges(edges);
        }
    }

    pub(super) fn install_web_ime(&mut self, window: &Window) {
        if self.web_ime.is_some() {
            return;
        }
        self.web_ime = window
            .canvas()
            .and_then(|canvas| WebIme::new(canvas, self.waker()));
        if self.web_ime.is_none() {
            tracing::warn!(target: "sigil::ime", "IME text area not created; composition is off");
        }
    }
}

fn find_canvas(id: &str) -> Option<HtmlCanvasElement> {
    web_sys::window()?
        .document()?
        .get_element_by_id(id)?
        .dyn_into::<HtmlCanvasElement>()
        .ok()
}
