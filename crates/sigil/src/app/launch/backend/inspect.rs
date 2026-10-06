use super::{Backend, Surface};
use crate::geometry::PhysicalSize;
use crate::renderer::RendererError;
use crate::renderer::capture::ImageTarget;
use crate::scene::CommandScene;
use crate::style::Color;

impl Backend {
    fn screenshot(
        &mut self,
        size: PhysicalSize,
        scene: &CommandScene,
        clear: Color,
        viewport: (f32, f32),
    ) -> Result<Vec<u8>, RendererError> {
        let image = self.render_image(
            scene,
            ImageTarget {
                size,
                viewport,
                clear,
            },
        )?;
        let mut png = Vec::new();
        image
            .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .map_err(|err| RendererError::Software(err.to_string()))?;
        Ok(png)
    }
}

pub(in crate::app::launch) fn inspect_shot(
    app: &mut crate::app::embed::Host,
    renderer: &mut Backend,
    surface: Option<&Surface>,
    viewport: (f32, f32),
) -> Result<serde_json::Value, String> {
    let Some((cx, _, root)) = app.tree() else {
        return Err("no mounted tree".into());
    };
    let scene =
        super::super::scene_cache::window_scene(cx, root, viewport, !renderer.is_transparent());
    let Some(surface) = surface else {
        return Err("window surface is not initialized yet".into());
    };
    let clear = renderer.clear_color();
    let size = surface_size(surface);
    let png = renderer
        .screenshot(size, &scene, clear, viewport)
        .map_err(|err| err.to_string())?;
    Ok(serde_json::json!({
        "png": base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            png,
        ),
        "width": size.width,
        "height": size.height,
    }))
}

const fn surface_size(surface: &Surface) -> PhysicalSize {
    match surface {
        Surface::Wgpu(surface) => surface.size(),
        Surface::Cpu(surface) => surface.size(),
    }
}
