use crate::Error;
use crate::renderer::capture::{ImageRenderer, ImageTarget, render_to_image};
use crate::scene::CommandScene;

use super::{Backend, BackendInner, RenderSurface};

impl Backend {
    pub(in crate::app) fn render_image(
        &mut self,
        scene: &CommandScene,
        target: ImageTarget,
    ) -> Result<image::RgbaImage, Error> {
        let (attempt, had_device) = match &mut self.inner {
            BackendInner::Cpu(r) => return render_to_image(ImageRenderer::Cpu(r), scene, target),
            BackendInner::Wgpu(r) => {
                let had_device = r.device().is_some();
                (
                    render_to_image(ImageRenderer::Gpu(r), scene, target),
                    had_device,
                )
            }
        };
        match attempt {
            Ok(image) => {
                self.adopt_gpu_status();
                Ok(image)
            }
            Err(err) if self.fallback && !had_device => {
                let mut cpu = self.fall_back_to_cpu(&err);
                let image = render_to_image(ImageRenderer::Cpu(&mut cpu), scene, target);
                self.use_cpu(cpu);
                image
            }
            Err(err) => Err(err),
        }
    }
}

pub(in crate::app::launch) fn inspect_shot(
    app: &mut crate::app::embed::Host,
    renderer: &mut Backend,
    surface: Option<&RenderSurface>,
    viewport: (f32, f32),
) -> Result<serde_json::Value, String> {
    let Some((cx, _, _, root)) = app.tree() else {
        return Err("no mounted tree".into());
    };
    let scene = crate::app::launch::scene_cache::window_scene(
        cx,
        root,
        viewport,
        !renderer.is_transparent(),
    );
    let size = match surface {
        Some(RenderSurface::Wgpu(surface)) => surface.size(),
        Some(RenderSurface::Cpu(surface)) => surface.size(),
        None => return Err("window surface is not initialized yet".into()),
    };
    let clear = renderer.clear_color();
    let image = renderer
        .render_image(
            &scene,
            ImageTarget {
                size,
                viewport,
                clear,
            },
        )
        .map_err(|err| err.to_string())?;
    let mut png = Vec::new();
    image
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .map_err(|err| format!("software renderer failed: {err}"))?;
    Ok(serde_json::json!({
        "png": base64::Engine::encode(&base64::engine::general_purpose::STANDARD, png),
        "width": size.width,
        "height": size.height,
    }))
}
