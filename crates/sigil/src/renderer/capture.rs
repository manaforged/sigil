use crate::geometry::PhysicalSize;
use crate::renderer::cpu::CpuRenderer;
#[cfg(any(test, feature = "inspect", feature = "kittest"))]
use crate::renderer::renderer::WgpuRenderer;
use crate::renderer::scene_command::CommandScene;
use crate::style::Rgba;
use crate::{Error, ErrorKind};

#[cfg(any(test, feature = "inspect", feature = "kittest"))]
mod gpu;
#[cfg(any(test, feature = "inspect", feature = "kittest"))]
pub use gpu::{CaptureExtent, FORMAT, capture_texture, read_texture};

#[derive(Debug)]
pub enum ImageRenderer<'a> {
    #[cfg(any(test, feature = "inspect", feature = "kittest"))]
    Gpu(&'a mut WgpuRenderer),
    Cpu(&'a mut CpuRenderer),
}

#[derive(Clone, Copy, Debug)]
pub struct ImageTarget {
    pub size: PhysicalSize,
    pub viewport: (f32, f32),
    pub clear: Rgba,
}

impl ImageTarget {
    #[allow(
        clippy::cast_precision_loss,
        reason = "image dimensions are far below the f32 mantissa limit"
    )]
    #[cfg(any(test, feature = "inspect", feature = "kittest"))]
    fn scale(self) -> f32 {
        self.size.width as f32 / self.viewport.0.max(1.0)
    }
}

pub fn render_to_image(
    renderer: ImageRenderer<'_>,
    scene: &CommandScene,
    target: ImageTarget,
) -> Result<image::RgbaImage, Error> {
    if target.size.width == 0 || target.size.height == 0 {
        return Err(Error::fixed(
            ErrorKind::InvalidViewport,
            "viewport dimensions must be strictly positive (width > 0, height > 0)",
        ));
    }
    let rgba = match renderer {
        #[cfg(any(test, feature = "inspect", feature = "kittest"))]
        ImageRenderer::Gpu(renderer) => gpu::gpu_pixels(renderer, scene, target)?,
        ImageRenderer::Cpu(renderer) => cpu_pixels(renderer, scene, target)?,
    };
    image::RgbaImage::from_raw(target.size.width, target.size.height, rgba).ok_or(Error::fixed(
        ErrorKind::InvalidViewport,
        "viewport dimensions must be strictly positive (width > 0, height > 0)",
    ))
}

fn cpu_pixels(
    renderer: &mut CpuRenderer,
    scene: &CommandScene,
    target: ImageTarget,
) -> Result<Vec<u8>, Error> {
    let pixmap = renderer.render(scene, target.clear, target.viewport, target.size)?;
    let mut rgba = pixmap.take();
    for pixel in rgba.as_chunks_mut::<4>().0 {
        if pixel[3] == u8::MAX {
            continue;
        }
        let premultiplied =
            tiny_skia::PremultipliedColorU8::from_rgba(pixel[0], pixel[1], pixel[2], pixel[3]);
        if let Some(color) = premultiplied.map(|color| color.demultiply()) {
            pixel.copy_from_slice(&[color.red(), color.green(), color.blue(), color.alpha()]);
        }
    }
    Ok(rgba)
}
