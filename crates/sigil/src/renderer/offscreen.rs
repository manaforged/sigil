use crate::style::Color;

use crate::renderer::device_state::PipelineCache;
use crate::renderer::error::RendererError;
use crate::renderer::frame_stats::FrameStats;
use crate::renderer::layer_pool::LayerHandle;
use crate::renderer::renderer::{ScreenSubmission, WgpuRenderer, record_clear};
use crate::renderer::scene_command::CommandScene;
use crate::renderer::scene_command::walk::{WalkCtx, walk_command_scene};
use crate::renderer::split_encoder::SplitEncoder;

#[derive(Clone, Copy)]
pub struct OffscreenTarget<'a> {
    pub device: &'a wgpu::Device,
    pub queue: &'a wgpu::Queue,
    pub view: &'a wgpu::TextureView,
    pub format: wgpu::TextureFormat,
    pub viewport: (f32, f32),
    pub scale: f32,
}

impl<'a> From<&ScreenSubmission<'a>> for OffscreenTarget<'a> {
    fn from(target: &ScreenSubmission<'a>) -> Self {
        Self {
            device: target.device,
            queue: target.queue,
            view: target.view,
            format: target.format,
            viewport: target.viewport,
            scale: target.scale,
        }
    }
}

impl WgpuRenderer {
    pub fn render_command_scene_offscreen(
        &mut self,
        target: OffscreenTarget<'_>,
        scene: &CommandScene,
        clear_color: Color,
    ) -> Result<(), RendererError> {
        self.last_frame_stats = FrameStats::default();
        let mut encoder = target
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("sigil-renderer offscreen encoder"),
            });
        record_clear(&mut encoder, target.view, clear_color);
        let mut encoder = SplitEncoder::new(target.device, target.queue, encoder);
        let stats = &mut self.last_frame_stats;
        let result = if let Ok(ready) = self.state.ready_mut()
            && ready.device == *target.device
        {
            ready
                .pipelines
                .walk(&mut encoder, &mut Vec::new(), target, scene, stats)
        } else {
            PipelineCache::default().walk(&mut encoder, &mut Vec::new(), target, scene, stats)
        };
        target
            .queue
            .submit(core::iter::once(encoder.into_inner().finish()));
        result
    }
}

impl PipelineCache {
    #[expect(
        clippy::too_many_arguments,
        reason = "the walk writes to the encoder, retained layers and stats"
    )]
    pub(crate) fn walk(
        &mut self,
        encoder: &mut SplitEncoder,
        retained: &mut Vec<LayerHandle>,
        target: OffscreenTarget<'_>,
        scene: &CommandScene,
        stats: &mut FrameStats,
    ) -> Result<(), RendererError> {
        let Self {
            scene_pipelines,
            fill_pipelines,
            shadow_pipelines,
            slug_pipelines,
            raster_pipelines,
            sprite_pipelines,
            decoration_pipelines,
            layer_pipelines,
            blur_pipelines,
            layer_pool,
            pending,
        } = self;
        walk_command_scene(
            WalkCtx {
                device: target.device,
                queue: target.queue,
                encoder,
                retained,
                view: target.view,
                viewport: target.viewport,
                scale: target.scale,
                format: target.format,
                scene_pipelines,
                fill_pipelines,
                shadow_pipelines,
                slug_pipelines,
                raster_pipelines,
                sprite_pipelines,
                decoration_pipelines,
                layer_pipelines,
                blur_pipelines,
                layer_pool,
                pending,
                stats,
            },
            scene,
        )
    }
}
