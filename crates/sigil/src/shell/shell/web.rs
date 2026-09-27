use wasm_bindgen::JsCast as _;
use web_sys::HtmlCanvasElement;
use winit::platform::web::WindowAttributesExtWebSys as _;
use winit::window::WindowAttributes;

use super::WinitShell;

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
}

fn find_canvas(id: &str) -> Option<HtmlCanvasElement> {
    web_sys::window()?
        .document()?
        .get_element_by_id(id)?
        .dyn_into::<HtmlCanvasElement>()
        .ok()
}
