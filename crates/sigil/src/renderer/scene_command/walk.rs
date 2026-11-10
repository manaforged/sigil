use crate::geometry::{Point, Rect};

use crate::Error;
use crate::renderer::decoration_paint::DecorationInstance;
use crate::renderer::decoration_paint::DecorationPipeline;
use crate::renderer::fill_paint::FillPipeline;
use crate::renderer::filter::FilterPipeline;
use crate::renderer::frame_stats::FrameStats;
use crate::renderer::frame_upload::{FrameUploads, UploadCapacities};
use crate::renderer::layer_composite::LayerCompositePipeline;
use crate::renderer::layer_pool::{LayerHandle, LayerPool};
use crate::renderer::plan::{self};
use crate::renderer::plan::{Batch, Flush, PlanStep};
use crate::renderer::projected::ProjectedPipeline;
use crate::renderer::quad_paint::QuadInstance;
use crate::renderer::quad_paint::{AtlasPipeline, QuadGroup};
use crate::renderer::scene_command::KeptLayers;
use crate::renderer::scene_command::state::SceneState;
use crate::renderer::scene_command::{CommandScene, SceneCommand, Transform};
use crate::renderer::scene_instance::{FillInstance, GpuStop, RectInstance, ShadowInstance};
use crate::renderer::scene_paint::ScenePipeline;
use crate::renderer::shadow_paint::ShadowPipeline;
use crate::renderer::slug::{QueuedRun, SlugTextPipeline};
use crate::renderer::split_encoder::SplitEncoder;
use crate::renderer::vector_pass::VectorPass;
use crate::renderer::wash_cache::WashCache;
pub(crate) use crate::scene::command::effects::transform_rect;
pub(crate) use crate::scene::geometry::{IDENTITY, compose, intersect_rect};

pub(crate) struct WalkCtx<'a> {
    pub(crate) device: &'a wgpu::Device,
    pub(crate) queue: &'a wgpu::Queue,
    pub(crate) encoder: &'a mut SplitEncoder,
    pub(crate) retained: &'a mut Vec<LayerHandle>,
    pub(crate) view: &'a wgpu::TextureView,
    pub(crate) viewport: (f32, f32),
    pub(crate) scale: f32,
    pub(crate) format: wgpu::TextureFormat,
    pub(crate) scene_pipelines: &'a mut Vec<ScenePipeline>,
    pub(crate) fill_pipelines: &'a mut Vec<FillPipeline>,
    pub(crate) shadow_pipelines: &'a mut Vec<ShadowPipeline>,
    pub(crate) slug_pipelines: &'a mut Vec<SlugTextPipeline>,
    pub(crate) quad_pipelines: &'a mut Vec<AtlasPipeline>,
    pub(crate) decoration_pipelines: &'a mut Vec<DecorationPipeline>,
    pub(crate) layer_pipelines: &'a mut Vec<LayerCompositePipeline>,
    pub(crate) filter_pipelines: &'a mut Vec<FilterPipeline>,
    pub(crate) projected_pipelines: &'a mut Vec<ProjectedPipeline>,
    pub(crate) layer_pool: &'a mut Option<LayerPool>,
    pub(crate) wash_cache: &'a mut WashCache,
    pub(crate) pending: &'a mut WalkPending,
    pub(crate) vector: &'a mut VectorPass,
    pub(crate) kept: &'a mut KeptLayers,
    pub(crate) stats: &'a mut FrameStats,
}

#[derive(Default)]
pub(crate) struct WalkPending {
    pub(super) rects: Vec<RectInstance>,
    pub(super) fills: Vec<FillInstance>,
    pub(super) stops: Vec<GpuStop>,
    pub(super) shadows: Vec<ShadowInstance>,
    pub(super) decorations: Vec<DecorationInstance>,
    pub(super) glyphs: Vec<QueuedGlyphs>,
    pub(super) runs: Vec<QueuedRun>,
    pub(super) raster: Vec<QuadInstance>,
    pub(super) raster_groups: Vec<QuadGroup>,
}

impl WalkPending {
    fn clear(&mut self) {
        self.rects.clear();
        self.fills.clear();
        self.stops.clear();
        self.shadows.clear();
        self.decorations.clear();
        self.glyphs.clear();
        self.runs.clear();
        self.raster.clear();
        self.raster_groups.clear();
    }
}

pub(crate) fn walk_command_scene(ctx: WalkCtx<'_>, scene: &CommandScene) -> Result<(), Error> {
    ctx.encoder.reserve_draws(&scene.commands);
    ctx.vector.begin_frame();
    ctx.kept.begin_frame();
    *ctx.stats = FrameStats {
        scene_commands: scene.commands.len(),
        ..FrameStats::default()
    };
    if let Some(pool) = ctx.layer_pool.as_mut() {
        pool.prepare(ctx.view.texture());
        let (washes, vector) = (&*ctx.wash_cache, &*ctx.vector);
        for pipeline in &mut *ctx.layer_pipelines {
            pipeline.retain(|view| {
                pool.contains(view) || washes.contains(view) || vector.contains(view)
            });
        }
    }
    if plan::needs_layers(&scene.commands) {
        return super::layer_walk::walk_layer_command_scene(ctx, scene);
    }
    let capacity = UploadCapacities::from_commands(scene.commands.iter());
    SceneWalk::new(ctx, capacity).walk(scene);
    Ok(())
}

pub(super) struct SceneWalk<'a> {
    pub(super) ctx: WalkCtx<'a>,
    pub(super) state: SceneState,
    pub(super) uploads: FrameUploads,
}

pub(crate) struct QueuedGlyphs {
    pub(super) run: QueuedRun,
    pub(super) raster: Vec<QuadInstance>,
    pub(super) bounds: Rect,
}

impl<'a> SceneWalk<'a> {
    pub(super) fn new(ctx: WalkCtx<'a>, capacity: UploadCapacities) -> Self {
        ctx.pending.clear();
        let uploads = FrameUploads::new(ctx.encoder, ctx.viewport, capacity, &mut *ctx.stats);
        Self {
            ctx,
            state: SceneState::default(),
            uploads,
        }
    }

    pub(super) fn walk(mut self, scene: &CommandScene) {
        for command in &scene.commands {
            self.handle(command);
        }
        self.finish();
    }

    pub(super) fn handle(&mut self, command: &SceneCommand) {
        match plan::step(&self.state, command, |rect| self.glyphs_under(rect)) {
            PlanStep::Skip => return,
            PlanStep::Queue {
                batch,
                flush_glyphs,
            } => {
                if flush_glyphs {
                    self.flush_glyphs();
                }
                self.queue(batch, command);
                return;
            }
            PlanStep::Dispatch(Flush::Shapes) => self.flush_shapes(),
            PlanStep::Dispatch(Flush::All) => self.finish(),
        }
        if self.state.apply(command) || self.state.clipped_out() {
            return;
        }
        match command {
            SceneCommand::Sprite(draw) => self.draw_sprite(draw),
            SceneCommand::Image(draw) => self.draw_image(draw),
            _ => {}
        }
    }

    fn queue(&mut self, batch: Batch, command: &SceneCommand) {
        if batch != Batch::Glyphs {
            self.flush_shapes_except(batch);
        }
        match command {
            SceneCommand::GlyphRun(draw) => match draw.fill() {
                Some(fill) => self.queue_path(fill),
                None => self.queue_glyph_run(draw),
            },
            SceneCommand::Rect(draw) => self.queue_rect(draw),
            SceneCommand::Gradient(draw) => self.queue_gradient(draw),
            SceneCommand::Pattern(draw) => self.queue_pattern(draw),
            SceneCommand::DrawShadow {
                rect,
                radii,
                shadow,
            } => self.queue_shadow(*rect, *radii, *shadow),
            SceneCommand::Decoration(draw) => self.queue_decoration(draw),
            SceneCommand::Path(draw) => self.queue_path(draw),
            _ => {}
        }
    }

    fn flush_shapes_except(&mut self, keep: Batch) {
        if keep != Batch::Rect {
            self.flush_rects();
        }
        if keep != Batch::Fill {
            self.flush_fills();
        }
        if keep != Batch::Shadow {
            self.flush_shadows();
        }
        if keep != Batch::Decoration {
            self.flush_decorations();
        }
        if keep != Batch::Path {
            self.flush_paths();
        }
    }

    pub(super) fn finish(&mut self) {
        self.flush_shapes();
        self.flush_glyphs();
    }

    pub(super) fn flush_shapes(&mut self) {
        self.flush_rects();
        self.flush_fills();
        self.flush_shadows();
        self.flush_decorations();
        self.flush_paths();
    }

    fn glyphs_under(&self, rect: Rect) -> bool {
        self.ctx
            .pending
            .glyphs
            .iter()
            .any(|queued| intersect_rect(queued.bounds, rect).is_some())
    }
}

pub(super) const fn translation(tx: f32, ty: f32) -> Transform {
    [[1.0, 0.0, tx], [0.0, 1.0, ty]]
}

pub(super) const fn rect_from_fields(fields: [f32; 4]) -> Rect {
    let [x, y, width, height] = fields;
    Rect::new(Point::new(x, y), width, height)
}

pub(super) type PipelineFns<P> = (
    fn(&P) -> wgpu::TextureFormat,
    fn(&wgpu::Device, wgpu::TextureFormat) -> P,
);

pub(super) fn pipeline_for<'a, P>(
    pipelines: &'a mut Vec<P>,
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    (of, new): PipelineFns<P>,
) -> &'a mut P {
    let index = pipelines
        .iter()
        .position(|p| of(p) == format)
        .unwrap_or_else(|| {
            pipelines.push(new(device, format));
            pipelines.len() - 1
        });
    pipelines
        .get_mut(index)
        .expect("pipeline index was just constructed from the vec")
}
