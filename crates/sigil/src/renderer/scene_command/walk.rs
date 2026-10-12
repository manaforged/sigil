use crate::geometry::{Point, Rect};

use crate::renderer::blur_paint::BlurPipeline;
use crate::renderer::decoration_paint::{DecorationInstance, DecorationPipeline};
use crate::renderer::error::RendererError;
use crate::renderer::fill_paint::FillPipeline;
use crate::renderer::frame_stats::FrameStats;
use crate::renderer::frame_upload::{FrameUploads, UploadCapacities};
use crate::renderer::layer_composite::LayerCompositePipeline;
use crate::renderer::layer_pool::{LayerHandle, LayerPool};
use crate::renderer::raster_paint::{RasterGroup, RasterInstance, RasterTextPipeline};
use crate::renderer::scene_command::state::SceneState;
use crate::renderer::scene_command::{CommandScene, SceneCommand, Transform};
use crate::renderer::scene_instance::{FillInstance, GpuStop, RectInstance, ShadowInstance};
use crate::renderer::scene_paint::ScenePipeline;
use crate::renderer::shadow_paint::ShadowPipeline;
use crate::renderer::slug::{QueuedRun, SlugTextPipeline};
use crate::renderer::split_encoder::SplitEncoder;
use crate::renderer::sprite_paint::SpritePipeline;
pub(crate) use crate::scene::command::effects::transform_rect;

pub(crate) const IDENTITY: Transform = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];

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
    pub(crate) raster_pipelines: &'a mut Vec<RasterTextPipeline>,
    pub(crate) sprite_pipelines: &'a mut Vec<SpritePipeline>,
    pub(crate) decoration_pipelines: &'a mut Vec<DecorationPipeline>,
    pub(crate) layer_pipelines: &'a mut Vec<LayerCompositePipeline>,
    pub(crate) blur_pipelines: &'a mut Vec<BlurPipeline>,
    pub(crate) layer_pool: &'a mut Option<LayerPool>,
    pub(crate) pending: &'a mut WalkPending,
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
    pub(super) raster: Vec<RasterInstance>,
    pub(super) raster_groups: Vec<RasterGroup>,
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

pub(crate) fn walk_command_scene(
    ctx: WalkCtx<'_>,
    scene: &CommandScene,
) -> Result<(), RendererError> {
    ctx.encoder.reserve_draws(&scene.commands);
    *ctx.stats = FrameStats {
        scene_commands: scene.commands.len(),
        ..FrameStats::default()
    };
    if let Some(pool) = ctx.layer_pool.as_mut() {
        pool.prepare(ctx.view.texture());
        for pipeline in &mut *ctx.layer_pipelines {
            pipeline.retain(pool);
        }
    }
    if scene
        .commands
        .iter()
        .any(|cmd| matches!(cmd, SceneCommand::PushLayer(_)))
        || SceneState::has_composited_group(&scene.commands)
    {
        return super::layer_walk::walk_layer_command_scene(ctx, scene);
    }
    let capacity = UploadCapacities::from_commands(scene.commands.iter());
    Walk::new(ctx, capacity).walk(scene);
    Ok(())
}

pub(super) struct Walk<'a> {
    pub(super) ctx: WalkCtx<'a>,
    pub(super) state: SceneState,
    pub(super) uploads: FrameUploads,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Shape {
    Rect,
    Fill,
    Shadow,
    Decoration,
}

pub(crate) struct QueuedGlyphs {
    pub(super) run: QueuedRun,
    pub(super) raster: Vec<RasterInstance>,
    pub(super) bounds: Rect,
}

impl<'a> Walk<'a> {
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
        if self.state.clipped_out() && SceneState::draws(command) {
            return;
        }
        if self.queue_batchable(command) {
            return;
        }
        match command {
            SceneCommand::PushClip(_) | SceneCommand::PopClip => self.flush_shapes(),
            _ => self.finish(),
        }
        if self.state.apply(command) || self.state.clipped_out() {
            return;
        }
        match command {
            SceneCommand::PushLayer(_)
            | SceneCommand::PopLayer
            | SceneCommand::PushClip(_)
            | SceneCommand::PopClip
            | SceneCommand::PushTransform(_)
            | SceneCommand::PopTransform
            | SceneCommand::SaveLayer { .. }
            | SceneCommand::RestoreLayer => {}
            SceneCommand::Rect(_)
            | SceneCommand::Gradient(_)
            | SceneCommand::Pattern(_)
            | SceneCommand::DrawShadow { .. }
            | SceneCommand::Decoration(_) => {
                unreachable!("batchable commands return before dispatch")
            }
            SceneCommand::GlyphRun(_) => unreachable!("glyph runs are queued before dispatch"),
            SceneCommand::Sprite(draw) => self.draw_sprite(draw),
            SceneCommand::Image(draw) => self.draw_image(draw),
        }
    }

    fn queue_batchable(&mut self, command: &SceneCommand) -> bool {
        let keep = match command {
            SceneCommand::GlyphRun(draw) => {
                self.queue_glyph_run(draw);
                return true;
            }
            SceneCommand::Rect(draw) => {
                if self.glyphs_under(transform_rect(self.state.current_transform(), draw.rect)) {
                    self.flush_glyphs();
                }
                Shape::Rect
            }
            SceneCommand::Gradient(_) | SceneCommand::Pattern(_) => Shape::Fill,
            SceneCommand::DrawShadow { .. } => Shape::Shadow,
            SceneCommand::Decoration(_) => Shape::Decoration,
            _ => return false,
        };
        if keep != Shape::Rect {
            self.flush_glyphs();
        }
        self.flush_shapes_except(keep);
        match command {
            SceneCommand::Rect(draw) => self.queue_rect(draw),
            SceneCommand::Gradient(draw) => self.queue_gradient(draw),
            SceneCommand::Pattern(draw) => self.queue_pattern(draw),
            SceneCommand::DrawShadow {
                rect,
                radii,
                shadow,
            } => self.queue_shadow(*rect, *radii, *shadow),
            SceneCommand::Decoration(draw) => self.queue_decoration(draw),
            _ => {}
        }
        true
    }

    fn flush_shapes_except(&mut self, keep: Shape) {
        if keep != Shape::Rect {
            self.flush_rects();
        }
        if keep != Shape::Fill {
            self.flush_fills();
        }
        if keep != Shape::Shadow {
            self.flush_shadows();
        }
        if keep != Shape::Decoration {
            self.flush_decorations();
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

pub(crate) fn compose(parent: Transform, child: Transform) -> Transform {
    [
        [
            parent[0][0].mul_add(child[0][0], parent[0][1] * child[1][0]),
            parent[0][0].mul_add(child[0][1], parent[0][1] * child[1][1]),
            parent[0][0].mul_add(child[0][2], parent[0][1].mul_add(child[1][2], parent[0][2])),
        ],
        [
            parent[1][0].mul_add(child[0][0], parent[1][1] * child[1][0]),
            parent[1][0].mul_add(child[0][1], parent[1][1] * child[1][1]),
            parent[1][0].mul_add(child[0][2], parent[1][1].mul_add(child[1][2], parent[1][2])),
        ],
    ]
}

pub(super) const fn rect_from_fields(fields: [f32; 4]) -> Rect {
    let [x, y, width, height] = fields;
    Rect::new(Point::new(x, y), width, height)
}

pub(crate) fn intersect_rect(left: Rect, right: Rect) -> Option<Rect> {
    let x0 = left.origin.x.max(right.origin.x);
    let y0 = left.origin.y.max(right.origin.y);
    let x1 = (left.origin.x + left.width).min(right.origin.x + right.width);
    let y1 = (left.origin.y + left.height).min(right.origin.y + right.height);
    (x1 > x0 && y1 > y0).then(|| Rect::new(Point::new(x0, y0), x1 - x0, y1 - y0))
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
