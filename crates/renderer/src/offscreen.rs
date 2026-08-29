use sigil_hal::Color;

use crate::blur_paint::BlurPipeline;
use crate::decoration_paint::DecorationPipeline;
use crate::device_state::ReadyState;
use crate::error::RendererError;
use crate::fill_paint::FillPipeline;
use crate::frame_stats::FrameStats;
use crate::layer_composite::LayerCompositePipeline;
use crate::layer_pool::{LayerHandle, LayerPool};
use crate::renderer::{ScreenSubmission, WgpuRenderer, record_clear};
use crate::scene_command::CommandScene;
use crate::scene_command::walk::{WalkCtx, walk_command_scene};
use crate::scene_paint::ScenePipeline;
use crate::shadow_paint::ShadowPipeline;
use crate::slug::SlugTextPipeline;
use crate::split_encoder::SplitEncoder;
use crate::sprite_paint::SpritePipeline;

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
        let Self {
            state,
            last_frame_stats,
            ..
        } = self;
        let ready = state.ensure_ready_headless()?;
        let mut encoder = target
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("sigil-renderer offscreen encoder"),
            });
        record_clear(&mut encoder, target.view, clear_color);
        let mut retained = Vec::new();
        let mut encoder = SplitEncoder::new(target.device, target.queue, encoder);
        let result = if ready.device == *target.device {
            walk_with_ready(
                ready,
                &mut encoder,
                &mut retained,
                target,
                scene,
                last_frame_stats,
            )
        } else {
            walk_with_temporary_pipelines(
                &mut encoder,
                &mut retained,
                target,
                scene,
                last_frame_stats,
            )
        };
        target
            .queue
            .submit(core::iter::once(encoder.into_inner().finish()));
        result
    }
}

#[allow(
    clippy::too_many_arguments,
    reason = "the argument list mirrors the upstream API contract"
)]
fn walk_with_ready(
    ready: &mut ReadyState,
    encoder: &mut SplitEncoder,
    retained: &mut Vec<LayerHandle>,
    target: OffscreenTarget<'_>,
    scene: &CommandScene,
    stats: &mut FrameStats,
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
            sprite_pipelines,
            decoration_pipelines,
            layer_pipelines,
            blur_pipelines,
            layer_pool,
            stats,
        },
        scene,
    )
}

#[allow(
    clippy::redundant_pub_crate,
    reason = "`offscreen` is a private module; pub(crate) reads cleaner than pub-and-then-re-export"
)]
pub(crate) fn walk_with_temporary_pipelines(
    encoder: &mut SplitEncoder,
    retained: &mut Vec<LayerHandle>,
    target: OffscreenTarget<'_>,
    scene: &CommandScene,
    stats: &mut FrameStats,
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
            encoder,
            retained,
            view: target.view,
            viewport: target.viewport,
            scale: target.scale,
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
            stats,
        },
        scene,
    )
}
