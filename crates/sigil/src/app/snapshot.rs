use crate::app::embed::Host;
use crate::app::error::AppResult;
use crate::app::launch::backend::Backend;
use crate::geometry::PhysicalSize;
use crate::kernel::IntoNode;
use crate::renderer::capture::ImageTarget;

pub fn snapshot(view: impl IntoNode, size: PhysicalSize) -> AppResult<image::RgbaImage> {
    let viewport = viewport_of(size);
    let mut host = Host::new();
    host.mount(view);
    host.publish_viewport(viewport);
    host.settle_layout(viewport)?;
    let mut scene = host.emit(viewport);
    crate::scene::paint_backdrop(&mut scene, host.theme(), viewport);
    let mut backend = Backend::from_env()?;
    let clear = backend.clear_color();
    Ok(backend.render_image(
        &scene,
        ImageTarget {
            size,
            viewport,
            clear,
        },
    )?)
}

#[allow(
    clippy::cast_precision_loss,
    reason = "image dimensions are far below the f32 mantissa limit"
)]
const fn viewport_of(size: PhysicalSize) -> (f32, f32) {
    (size.width as f32, size.height as f32)
}
