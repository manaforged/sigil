//! Command-scene interpreter for `WgpuRenderer`.

use sigil_hal::{Color, Point, Rect};

use crate::error::RendererError;
use crate::scene_command::{
    CommandScene, DrawGlyphRun, DrawRect, GlyphRun, SceneCommand, Transform,
};
use crate::scene_instance::draw_rect_instance;
use crate::scene_paint::{PaintCtx, ScenePipeline};
use crate::slug::{DrawParams, SlugTextPipeline};

const IDENTITY: Transform = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];

pub(crate) struct WalkCtx<'a> {
    pub(crate) device: &'a wgpu::Device,
    pub(crate) queue: &'a wgpu::Queue,
    pub(crate) view: &'a wgpu::TextureView,
    pub(crate) viewport: (f32, f32),
    pub(crate) format: wgpu::TextureFormat,
    pub(crate) scene_pipelines: &'a mut Vec<ScenePipeline>,
    pub(crate) slug_pipelines: &'a mut Vec<SlugTextPipeline>,
}

pub(crate) fn walk_command_scene(
    ctx: WalkCtx<'_>,
    scene: &CommandScene,
) -> Result<(), RendererError> {
    Walk::new(ctx).walk(scene)
}

struct Walk<'a> {
    ctx: WalkCtx<'a>,
    clips: Vec<Rect>,
    transforms: Vec<Transform>,
    layers: Vec<f32>,
}

impl<'a> Walk<'a> {
    fn new(ctx: WalkCtx<'a>) -> Self {
        Self {
            ctx,
            clips: Vec::new(),
            transforms: vec![IDENTITY],
            layers: vec![1.0],
        }
    }

    fn walk(mut self, scene: &CommandScene) -> Result<(), RendererError> {
        for command in scene.commands() {
            self.handle(command)?;
        }
        Ok(())
    }

    fn handle(&mut self, command: &SceneCommand) -> Result<(), RendererError> {
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
            SceneCommand::SaveLayer { opacity } => self.push_layer(*opacity),
            SceneCommand::RestoreLayer => {
                if self.layers.len() > 1 {
                    let _ = self.layers.pop();
                }
            }
            SceneCommand::DrawRect(draw) => self.draw_rect(draw),
            SceneCommand::DrawGlyphRun(draw) => self.draw_glyph_run(draw)?,
        }
        Ok(())
    }

    fn push_clip(&mut self, rect: Rect) {
        self.clips
            .push(transform_rect(self.current_transform(), rect));
    }

    fn push_transform(&mut self, transform: Transform) {
        self.transforms
            .push(compose(self.current_transform(), transform));
    }

    fn push_layer(&mut self, opacity: f32) {
        self.layers
            .push(self.current_opacity() * opacity.clamp(0.0, 1.0));
    }

    fn draw_rect(&mut self, draw: &DrawRect) {
        let mut transformed = *draw;
        transformed.rect = transform_rect(self.current_transform(), draw.rect);
        let Some(rect) = self.clip_rect(transformed.rect) else {
            return;
        };
        transformed.rect = rect;
        let instance = draw_rect_instance(&transformed, self.current_opacity());
        let mut encoder = self
            .ctx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("sigil-renderer command-scene rect encoder"),
            });
        let pipeline =
            scene_pipeline_for(self.ctx.scene_pipelines, self.ctx.device, self.ctx.format);
        pipeline.paint_instances(
            PaintCtx {
                device: self.ctx.device,
                queue: self.ctx.queue,
                encoder: &mut encoder,
                view: self.ctx.view,
                viewport: self.ctx.viewport,
            },
            core::slice::from_ref(&instance),
        );
        self.ctx.queue.submit(core::iter::once(encoder.finish()));
    }

    fn draw_glyph_run(&mut self, draw: &DrawGlyphRun) -> Result<(), RendererError> {
        let run = &draw.run;
        let rect = transform_rect(self.current_transform(), run.rect);
        if self.clip_rect(rect).is_none() {
            return Ok(());
        }
        let Some(paragraph) = run.paragraph() else {
            return Ok(());
        };
        let color = glyph_run_color(run, self.current_opacity());
        let pipeline = slug_pipeline_for(self.ctx.slug_pipelines, self.ctx.device, self.ctx.format);
        pipeline.draw_paragraph(&DrawParams {
            device: self.ctx.device,
            queue: self.ctx.queue,
            view: self.ctx.view,
            surface_width: self.ctx.viewport.0,
            surface_height: self.ctx.viewport.1,
            paragraph,
            glyphs: Some(run.positioned_glyphs()),
            origin: (rect.origin.x, rect.origin.y),
            color: Some(color),
        })
    }

    fn clip_rect(&self, rect: Rect) -> Option<Rect> {
        self.clips.iter().copied().try_fold(rect, intersect_rect)
    }

    fn current_transform(&self) -> Transform {
        self.transforms.last().copied().unwrap_or(IDENTITY)
    }

    fn current_opacity(&self) -> f32 {
        self.layers.last().copied().unwrap_or(1.0)
    }
}

fn scene_pipeline_for<'a>(
    pipelines: &'a mut Vec<ScenePipeline>,
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
) -> &'a mut ScenePipeline {
    let idx = pipelines
        .iter()
        .position(|p| p.format() == format)
        .unwrap_or_else(|| {
            pipelines.push(ScenePipeline::new(device, format));
            pipelines.len() - 1
        });
    pipelines
        .get_mut(idx)
        .expect("scene pipeline index was just constructed from the vec")
}

fn slug_pipeline_for<'a>(
    pipelines: &'a mut Vec<SlugTextPipeline>,
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
) -> &'a mut SlugTextPipeline {
    let idx = pipelines
        .iter()
        .position(|p| p.format() == format)
        .unwrap_or_else(|| {
            pipelines.push(SlugTextPipeline::new(device, format));
            pipelines.len() - 1
        });
    pipelines
        .get_mut(idx)
        .expect("slug pipeline index was just constructed from the vec")
}

fn glyph_run_color(run: &GlyphRun, opacity_scale: f32) -> Color {
    let opacity = (f32::from(run.opacity) / 10_000.0) * opacity_scale;
    Color::rgba(
        f32::from(run.color.r) / 255.0,
        f32::from(run.color.g) / 255.0,
        f32::from(run.color.b) / 255.0,
        (f32::from(run.color.a) / 255.0) * opacity.clamp(0.0, 1.0),
    )
}

fn compose(parent: Transform, child: Transform) -> Transform {
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

fn transform_rect(transform: Transform, rect: Rect) -> Rect {
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

fn transform_point(transform: Transform, point: Point) -> Point {
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

fn intersect_rect(left: Rect, right: Rect) -> Option<Rect> {
    let x0 = left.origin.x.max(right.origin.x);
    let y0 = left.origin.y.max(right.origin.y);
    let x1 = (left.origin.x + left.width).min(right.origin.x + right.width);
    let y1 = (left.origin.y + left.height).min(right.origin.y + right.height);
    (x1 > x0 && y1 > y0).then(|| Rect::new(Point::new(x0, y0), x1 - x0, y1 - y0))
}
