use crate::hal::{Point, Rect};

use crate::renderer::blur_paint::BlurPipeline;
use crate::renderer::decoration_paint::{DecorationInstance, DecorationPipeline};
use crate::renderer::error::RendererError;
use crate::renderer::fill_paint::FillPipeline;
use crate::renderer::frame_stats::FrameStats;
use crate::renderer::frame_upload::{FrameUploads, UploadCapacities};
use crate::renderer::layer_composite::LayerCompositePipeline;
use crate::renderer::layer_pool::{LayerHandle, LayerPool};
use crate::renderer::raster_paint::RasterTextPipeline;
use crate::renderer::scene_command::{CommandScene, SceneCommand, Transform};
use crate::renderer::scene_instance::{FillInstance, GpuStop, RectInstance, ShadowInstance};
use crate::renderer::scene_paint::ScenePipeline;
use crate::renderer::shadow_paint::ShadowPipeline;
use crate::renderer::slug::{QueuedRun, SlugTextPipeline};
use crate::renderer::split_encoder::SplitEncoder;
use crate::renderer::sprite_paint::SpritePipeline;

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
    pub(crate) stats: &'a mut FrameStats,
}

pub(crate) fn walk_command_scene(
    ctx: WalkCtx<'_>,
    scene: &CommandScene,
) -> Result<(), RendererError> {
    *ctx.stats = FrameStats {
        scene_commands: scene.commands.len(),
        ..FrameStats::default()
    };
    if scene
        .commands
        .iter()
        .any(|cmd| matches!(cmd, SceneCommand::PushLayer(_)))
    {
        return super::layer_walk::walk_layer_command_scene(ctx, scene);
    }
    let capacity = UploadCapacities::from_commands(scene.commands.iter());
    Walk::new(ctx, capacity).walk(scene);
    Ok(())
}

pub(super) struct Walk<'a> {
    pub(super) ctx: WalkCtx<'a>,
    clips: Vec<Rect>,
    transforms: Vec<Transform>,
    layers: Vec<f32>,
    pub(super) uploads: FrameUploads,
    pub(super) pending_rects: Vec<RectInstance>,
    pub(super) pending_fills: Vec<FillInstance>,
    pub(super) pending_stops: Vec<GpuStop>,
    pub(super) pending_shadows: Vec<ShadowInstance>,
    pub(super) pending_decorations: Vec<DecorationInstance>,
    pub(super) pending_glyphs: Vec<QueuedGlyphs>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Shape {
    Rect,
    Fill,
    Shadow,
    Decoration,
}

pub(super) struct QueuedGlyphs {
    pub(super) run: QueuedRun,
    pub(super) bounds: Rect,
}

impl<'a> Walk<'a> {
    pub(super) fn new(ctx: WalkCtx<'a>, capacity: UploadCapacities) -> Self {
        let uploads = FrameUploads::new(
            ctx.device,
            ctx.queue,
            ctx.viewport,
            capacity,
            &mut *ctx.stats,
        );
        Self {
            ctx,
            clips: Vec::new(),
            transforms: vec![IDENTITY],
            layers: vec![1.0],
            uploads,
            pending_rects: Vec::new(),
            pending_fills: Vec::new(),
            pending_stops: Vec::new(),
            pending_shadows: Vec::new(),
            pending_decorations: Vec::new(),
            pending_glyphs: Vec::new(),
        }
    }

    pub(super) fn walk(mut self, scene: &CommandScene) {
        for command in &scene.commands {
            self.handle(command);
        }
        self.finish();
    }

    pub(super) fn handle(&mut self, command: &SceneCommand) {
        if self.queue_batchable(command) {
            return;
        }
        match command {
            SceneCommand::PushClip(_) | SceneCommand::PopClip => self.flush_shapes(),
            _ => self.finish(),
        }
        match command {
            SceneCommand::PushClip(rect) => self.push_clip(*rect),
            SceneCommand::PopClip => {
                let _ = self.clips.pop();
            }
            SceneCommand::PushTransform(transform) => self.push_transform(*transform),
            SceneCommand::PopTransform => {
                if self.transforms.len() > 1 {
                    let _ = self.transforms.pop();
                }
            }
            SceneCommand::SaveLayer { opacity } => self.push_opacity_layer(*opacity),
            SceneCommand::RestoreLayer => {
                if self.layers.len() > 1 {
                    let _ = self.layers.pop();
                }
            }
            SceneCommand::PushLayer(_) | SceneCommand::PopLayer => {}
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
        match command {
            SceneCommand::GlyphRun(draw) => {
                self.queue_glyph_run(draw);
                true
            }
            SceneCommand::Rect(draw) => {
                if self.glyphs_under(transform_rect(self.current_transform(), draw.rect)) {
                    self.flush_glyphs();
                }
                self.flush_shapes_except(Shape::Rect);
                self.queue_rect(draw);
                true
            }
            SceneCommand::Gradient(draw) => {
                self.flush_glyphs();
                self.flush_shapes_except(Shape::Fill);
                self.queue_gradient(draw);
                true
            }
            SceneCommand::Pattern(draw) => {
                self.flush_glyphs();
                self.flush_shapes_except(Shape::Fill);
                self.queue_pattern(draw);
                true
            }
            SceneCommand::DrawShadow {
                rect,
                radii,
                shadow,
            } => {
                self.flush_glyphs();
                self.flush_shapes_except(Shape::Shadow);
                self.queue_shadow(*rect, *radii, *shadow);
                true
            }
            SceneCommand::Decoration(draw) => {
                self.flush_glyphs();
                self.flush_shapes_except(Shape::Decoration);
                self.queue_decoration(draw);
                true
            }
            _ => false,
        }
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
        self.pending_glyphs
            .iter()
            .any(|queued| intersect_rect(queued.bounds, rect).is_some())
    }

    pub(super) fn push_clip(&mut self, rect: Rect) {
        self.clips
            .push(transform_rect(self.current_transform(), rect));
    }

    pub(super) fn push_transform(&mut self, transform: Transform) {
        self.transforms
            .push(compose(self.current_transform(), transform));
    }

    fn push_opacity_layer(&mut self, opacity: f32) {
        self.layers
            .push(self.current_opacity() * opacity.clamp(0.0, 1.0));
    }

    pub(super) fn clip_rect(&self, rect: Rect) -> Option<Rect> {
        self.clips.iter().copied().try_fold(rect, intersect_rect)
    }

    pub(super) fn current_clip(&self) -> Option<Rect> {
        let mut clips = self.clips.iter().copied();
        let first = clips.next()?;
        clips.try_fold(first, intersect_rect)
    }

    pub(super) fn current_transform(&self) -> Transform {
        self.transforms.last().copied().unwrap_or(IDENTITY)
    }

    pub(super) fn is_identity_transform(&self) -> bool {
        self.current_transform() == IDENTITY
    }

    pub(super) fn current_opacity(&self) -> f32 {
        self.layers.last().copied().unwrap_or(1.0)
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

pub(crate) fn transform_rect(transform: Transform, rect: Rect) -> Rect {
    let p0 = transform_point(transform, rect.origin);
    let p1 = transform_point(
        transform,
        Point::new(rect.origin.x + rect.width, rect.origin.y),
    );
    let p2 = transform_point(
        transform,
        Point::new(rect.origin.x, rect.origin.y + rect.height),
    );
    let p3 = transform_point(
        transform,
        Point::new(rect.origin.x + rect.width, rect.origin.y + rect.height),
    );
    bounding_rect([p0, p1, p2, p3])
}

#[allow(
    clippy::missing_const_for_fn,
    reason = "const f32::mul_add requires Rust 1.94; Sigil supports 1.90"
)]
pub(super) fn transform_point(transform: Transform, point: Point) -> Point {
    Point::new(
        transform[0][0].mul_add(point.x, transform[0][1].mul_add(point.y, transform[0][2])),
        transform[1][0].mul_add(point.x, transform[1][1].mul_add(point.y, transform[1][2])),
    )
}

fn bounding_rect(points: [Point; 4]) -> Rect {
    let min_x = points.iter().map(|p| p.x).fold(f32::INFINITY, f32::min);
    let max_x = points.iter().map(|p| p.x).fold(f32::NEG_INFINITY, f32::max);
    let min_y = points.iter().map(|p| p.y).fold(f32::INFINITY, f32::min);
    let max_y = points.iter().map(|p| p.y).fold(f32::NEG_INFINITY, f32::max);
    Rect::new(Point::new(min_x, min_y), max_x - min_x, max_y - min_y)
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
