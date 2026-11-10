use std::num::NonZeroU32;
use std::sync::Arc;

use crate::geometry::{PhysicalSize, Point};
use crate::hal::Renderer;
use crate::style::Rgba;
use tiny_skia::Pixmap;

use crate::renderer::frame_stats::FrameStats;
use crate::renderer::plan::build_tree;
use crate::renderer::scene_command::CommandScene;
use crate::renderer::scene_command::state::SceneState;
use crate::renderer::window_handle::{DisplayAndWindow, WindowHandle};
use crate::{Error, ErrorKind};

mod bound;
mod clip;
mod draws;
mod encoding;
mod filter;
mod glyph_outline;
mod layer_cache;
mod layers;
mod paint;
mod path;
mod pattern;
mod projected;
mod quad;
mod shadow;
mod shadow_field;
mod shapes;
mod sprite_tint;
mod stroke;

use paint::CpuPainter;
use quad::QuadCache;

type Shared = Arc<Box<dyn DisplayAndWindow>>;

#[derive(Default)]
pub struct CpuRenderer {
    quads: QuadCache,
    cached: layer_cache::LayerCache,
    layers: Vec<Pixmap>,
    covers: Vec<tiny_skia::Mask>,
    transparent: bool,
    pub(crate) stats: FrameStats,
}

impl core::fmt::Debug for CpuRenderer {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("CpuRenderer")
            .field("transparent", &self.transparent)
            .field("stats", &self.stats)
            .finish_non_exhaustive()
    }
}

pub struct CpuSurface {
    _context: softbuffer::Context<Shared>,
    surface: softbuffer::Surface<Shared, Shared>,
    size: PhysicalSize,
}

impl core::fmt::Debug for CpuSurface {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("CpuSurface")
            .field("size", &self.size)
            .finish_non_exhaustive()
    }
}

impl CpuSurface {
    #[cfg(feature = "inspect")]
    #[must_use]
    pub(crate) const fn size(&self) -> PhysicalSize {
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
        clear: Rgba,
        viewport: (f32, f32),
        size: PhysicalSize,
    ) -> Result<Pixmap, Error> {
        let mut target = Pixmap::new(size.width, size.height).ok_or(Error::fixed(
            ErrorKind::InvalidViewport,
            "viewport dimensions must be strictly positive (width > 0, height > 0)",
        ))?;
        target.fill(shapes::color(clear));
        self.paint(&mut target, scene, viewport);
        Ok(target)
    }

    /// Paints `scene` over the pixels already in `target`.
    fn paint(&mut self, target: &mut Pixmap, scene: &CommandScene, viewport: (f32, f32)) {
        let size = PhysicalSize::new(target.width(), target.height());
        self.walk(
            target,
            scene,
            physical_scale(size, viewport),
            Point::new(0.0, 0.0),
        );
    }

    #[cfg(feature = "terminal")]
    /// Repaints the `[x, y, w, h]` pixel region of `canvas` from `scene`, with masks sized to the region.
    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_wrap,
        reason = "pixel coordinates of a frame fit f32 and i32"
    )]
    pub(crate) fn render_region(
        &mut self,
        scene: &CommandScene,
        (viewport, clear): ((f32, f32), Rgba),
        size: PhysicalSize,
        [x, y, w, h]: [u32; 4],
    ) -> Option<Pixmap> {
        let mut region = Pixmap::new(w, h)?;
        region.fill(shapes::color(clear));
        let origin = Point::new(x as f32, y as f32);
        self.walk(&mut region, scene, physical_scale(size, viewport), origin);
        Some(region)
    }

    fn walk(&mut self, target: &mut Pixmap, scene: &CommandScene, scale: f32, origin: Point) {
        self.stats = FrameStats::default();
        self.quads.begin_frame();
        self.cached.begin_frame();
        self.stats.scene_commands = scene.commands.len();
        let scratch = paint::PaintScratch {
            quads: &mut self.quads,
            cached: &mut self.cached,
            stats: &mut self.stats,
            pool: &mut self.layers,
            covers: &mut self.covers,
        };
        CpuPainter::new(target, scale, origin, SceneState::default(), scratch)
            .walk(&build_tree(&scene.commands));
    }

    pub fn render_command_scene(
        &mut self,
        surface: &mut CpuSurface,
        scene: &CommandScene,
        clear: Rgba,
        viewport: (f32, f32),
    ) -> Result<(), Error> {
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
    type Error = Error;
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

fn software(err: &softbuffer::SoftBufferError) -> Error {
    Error::message(
        ErrorKind::Software,
        format_args!("software renderer failed: {err}"),
    )
}

#[cfg(test)]
#[path = "cpu_tests.rs"]
mod tests;
