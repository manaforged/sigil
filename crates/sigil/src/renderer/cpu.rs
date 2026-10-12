use std::num::NonZeroU32;
use std::sync::Arc;

use crate::geometry::{PhysicalSize, Point};
use crate::hal::Renderer;
use crate::style::Color;
use tiny_skia::Pixmap;

use crate::renderer::error::RendererError;
use crate::renderer::frame_stats::FrameStats;
use crate::renderer::scene_command::CommandScene;
use crate::renderer::scene_command::state::SceneState;
use crate::renderer::window_handle::{DisplayAndWindow, WindowHandle};

mod bound;
mod draws;
mod glyph_outline;
mod glyphs;
mod layers;
mod paint;
mod pattern;
mod raster;
mod shadow;
mod shadow_field;
mod shapes;
mod sprite_tint;
mod stroke;

use glyphs::GlyphCache;
use paint::Painter;
use raster::RasterCache;

type Shared = Arc<Box<dyn DisplayAndWindow>>;

#[derive(Default)]
pub struct CpuRenderer {
    glyphs: GlyphCache,
    raster: RasterCache,
    layers: Vec<Pixmap>,
    covers: Vec<tiny_skia::Mask>,
    transparent: bool,
    pub(crate) stats: FrameStats,
}

pub struct CpuSurface {
    _context: softbuffer::Context<Shared>,
    surface: softbuffer::Surface<Shared, Shared>,
    size: PhysicalSize,
}

impl CpuSurface {
    #[must_use]
    pub const fn size(&self) -> PhysicalSize {
        self.size
    }
}

impl CpuRenderer {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub const fn set_transparent(&mut self, transparent: bool) {
        self.transparent = transparent;
    }

    #[must_use]
    pub const fn is_transparent(&self) -> bool {
        self.transparent
    }

    #[cfg(any(test, feature = "kittest"))]
    #[must_use]
    pub const fn last_frame_stats(&self) -> FrameStats {
        self.stats
    }

    pub fn render(
        &mut self,
        scene: &CommandScene,
        clear: Color,
        viewport: (f32, f32),
        size: PhysicalSize,
    ) -> Result<Pixmap, RendererError> {
        let mut target =
            Pixmap::new(size.width, size.height).ok_or(RendererError::InvalidViewport)?;
        target.fill(shapes::color(clear));
        let scale = physical_scale(size, viewport);
        self.stats = FrameStats::default();
        self.raster.begin_frame();
        self.glyphs.begin_frame();
        self.stats.scene_commands = scene.commands.len();
        let scratch = paint::Scratch {
            glyphs: &mut self.glyphs,
            raster: &mut self.raster,
            stats: &mut self.stats,
            pool: &mut self.layers,
            covers: &mut self.covers,
        };
        Painter::new(
            &mut target,
            scale,
            Point::new(0.0, 0.0),
            SceneState::default(),
            scratch,
        )
        .walk(&scene.commands);
        Ok(target)
    }

    pub fn render_command_scene(
        &mut self,
        surface: &mut CpuSurface,
        scene: &CommandScene,
        clear: Color,
        viewport: (f32, f32),
    ) -> Result<(), RendererError> {
        let pixmap = self.render(scene, clear, viewport, surface.size)?;
        let mut buffer = surface.surface.buffer_mut().map_err(|err| software(&err))?;
        for (out, px) in buffer.iter_mut().zip(pixmap.pixels()) {
            *out =
                (u32::from(px.red()) << 16) | (u32::from(px.green()) << 8) | u32::from(px.blue());
        }
        buffer.present().map_err(|err| software(&err))
    }
}

impl Renderer for CpuRenderer {
    type Error = RendererError;
    type Surface = CpuSurface;
    type WindowHandle = WindowHandle;

    fn create_surface(
        &mut self,
        window: Self::WindowHandle,
        initial_size: PhysicalSize,
    ) -> Result<Self::Surface, Self::Error> {
        let shared: Shared = Arc::new(window.into_inner());
        let context =
            softbuffer::Context::new(Arc::clone(&shared)).map_err(|err| software(&err))?;
        let surface = softbuffer::Surface::new(&context, shared).map_err(|err| software(&err))?;
        let mut out = CpuSurface {
            _context: context,
            surface,
            size: PhysicalSize::new(0, 0),
        };
        self.resize(&mut out, initial_size)?;
        Ok(out)
    }

    fn resize(
        &mut self,
        surface: &mut Self::Surface,
        size: PhysicalSize,
    ) -> Result<(), Self::Error> {
        let (width, height) = (nonzero(size.width), nonzero(size.height));
        surface
            .surface
            .resize(width, height)
            .map_err(|err| software(&err))?;
        surface.size = PhysicalSize::new(width.get(), height.get());
        Ok(())
    }

    #[allow(
        clippy::cast_precision_loss,
        reason = "surface dimensions are far below the f32 mantissa limit"
    )]
    fn clear(&mut self, surface: &mut Self::Surface, color: Color) -> Result<(), Self::Error> {
        let viewport = (surface.size.width as f32, surface.size.height as f32);
        self.render_command_scene(surface, &CommandScene::default(), color, viewport)
    }
}

fn nonzero(value: u32) -> NonZeroU32 {
    NonZeroU32::new(value.max(1)).unwrap_or(NonZeroU32::MIN)
}

#[allow(
    clippy::cast_precision_loss,
    reason = "surface dimensions are far below the f32 mantissa limit"
)]
fn physical_scale(size: PhysicalSize, viewport: (f32, f32)) -> f32 {
    size.width as f32 / viewport.0.max(1.0)
}

fn software(err: &softbuffer::SoftBufferError) -> RendererError {
    RendererError::Software(err.to_string())
}

#[cfg(test)]
#[path = "cpu_tests.rs"]
mod tests;
