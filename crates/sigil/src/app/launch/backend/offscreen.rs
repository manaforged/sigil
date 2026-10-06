use super::{Backend, Inner};
use crate::renderer::RendererError;
use crate::renderer::capture::{ImageRenderer, ImageTarget, render_to_image};
use crate::scene::CommandScene;

impl Backend {
    pub(in crate::app) fn render_image(
        &mut self,
        scene: &CommandScene,
        target: ImageTarget,
    ) -> Result<image::RgbaImage, RendererError> {
        let (attempt, had_device) = match &mut self.inner {
            Inner::Cpu(r) => return render_to_image(ImageRenderer::Cpu(r), scene, target),
            Inner::Wgpu(r) => {
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
                self.inner = Inner::Cpu(Box::new(cpu));
                image
            }
            Err(err) => Err(err),
        }
    }
}
