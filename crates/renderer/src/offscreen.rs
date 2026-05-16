//! Offscreen command-scene rendering support.

use sigil_hal::Color;

use crate::blur_paint::BlurPipeline;
use crate::decoration_paint::DecorationPipeline;
use crate::device_state::ReadyState;
use crate::error::RendererError;
use crate::fill_paint::FillPipeline;
use crate::layer_composite::LayerCompositePipeline;
use crate::layer_pool::LayerPool;
use crate::renderer::{WgpuRenderer, submit_clear};
use crate::scene_command::CommandScene;
use crate::scene_command::walk::{WalkCtx, walk_command_scene};
use crate::scene_paint::ScenePipeline;
use crate::shadow_paint::ShadowPipeline;
use crate::slug::SlugTextPipeline;
use crate::sprite_paint::SpritePipeline;

/// Wgpu render target for command-scene offscreen capture.
#[derive(Clone, Copy)]
pub struct OffscreenTarget<'a> {
    /// Device that owns `view`.
    pub device: &'a wgpu::Device,
    /// Queue paired with `device`.
    pub queue: &'a wgpu::Queue,
    /// Texture view to clear and draw into.
    pub view: &'a wgpu::TextureView,
    /// Texture format used to build render pipelines.
    pub format: wgpu::TextureFormat,
    /// Logical viewport used by scene-to-NDC conversion.
    pub viewport: (f32, f32),
}

impl WgpuRenderer {
    /// Clear `target.view`, then draw every command from `scene` into it.
    ///
    /// Unlike [`Self::render_command_scene`], this path does not acquire or
    /// present a window surface. It also promotes an uninitialized renderer to
    /// a headless device state, so callers do not need to create a surface
    /// before capturing into their own texture.
    pub fn render_command_scene_offscreen(
        &mut self,
        target: OffscreenTarget<'_>,
        scene: &CommandScene,
        clear_color: Color,
    ) -> Result<(), RendererError> {
        let ready = self.state.ensure_ready_headless()?;
        submit_clear(target.device, target.queue, target.view, clear_color);
        if ready.device == *target.device {
            walk_with_ready(ready, target, scene)
        } else {
            walk_with_temporary_pipelines(target, scene)
        }
    }
}

fn walk_with_ready(
    ready: &mut ReadyState,
    target: OffscreenTarget<'_>,
    scene: &CommandScene,
) -> Result<(), RendererError> {
    let ReadyState {
        scene_pipelines,
        fill_pipelines,
        shadow_pipelines,
        slug_pipelines,
        sprite_pipelines,
        decoration_pipelines,
        layer_pipelines,
        blur_pipelines,
        layer_pool,
        ..
    } = ready;
    walk_command_scene(
        WalkCtx {
            device: target.device,
            queue: target.queue,
            view: target.view,
            viewport: target.viewport,
            format: target.format,
            scene_pipelines,
            fill_pipelines,
            shadow_pipelines,
            slug_pipelines,
            sprite_pipelines,
            decoration_pipelines,
            layer_pipelines,
            blur_pipelines,
            layer_pool,
        },
        scene,
    )
}

#[allow(
    clippy::redundant_pub_crate,
    reason = "`offscreen` is a private module; pub(crate) reads cleaner than pub-and-then-re-export"
)]
pub(crate) fn walk_with_temporary_pipelines(
    target: OffscreenTarget<'_>,
    scene: &CommandScene,
) -> Result<(), RendererError> {
    let mut scene_pipelines = Vec::<ScenePipeline>::new();
    let mut fill_pipelines = Vec::<FillPipeline>::new();
    let mut shadow_pipelines = Vec::<ShadowPipeline>::new();
    let mut slug_pipelines = Vec::<SlugTextPipeline>::new();
    let mut sprite_pipelines = Vec::<SpritePipeline>::new();
    let mut decoration_pipelines = Vec::<DecorationPipeline>::new();
    let mut layer_pipelines = Vec::<LayerCompositePipeline>::new();
    let mut blur_pipelines = Vec::<BlurPipeline>::new();
    let mut layer_pool = Some(LayerPool::new());
    walk_command_scene(
        WalkCtx {
            device: target.device,
            queue: target.queue,
            view: target.view,
            viewport: target.viewport,
            format: target.format,
            scene_pipelines: &mut scene_pipelines,
            fill_pipelines: &mut fill_pipelines,
            shadow_pipelines: &mut shadow_pipelines,
            slug_pipelines: &mut slug_pipelines,
            sprite_pipelines: &mut sprite_pipelines,
            decoration_pipelines: &mut decoration_pipelines,
            layer_pipelines: &mut layer_pipelines,
            blur_pipelines: &mut blur_pipelines,
            layer_pool: &mut layer_pool,
        },
        scene,
    )
}
