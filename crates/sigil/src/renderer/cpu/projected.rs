#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    reason = "pixel coordinates are bounded by the target size; float-to-int rounding is the rasterizer contract"
)]

use crate::geometry::{Point, Rect};

use crate::renderer::plan::PlanNode;
use crate::renderer::projected::ProjectedFrame;
use crate::renderer::scene_command::LayerScope;
use crate::scene::command::BlendMode;
use crate::renderer::scene_command::state::SceneState;
use crate::scene::command::transform3d::{apply, invert};

use super::paint::CpuPainter;

type Plane = [[f32; 3]; 3];

struct Source<'a> {
    pixels: &'a [u8],
    width: usize,
    height: usize,
    bounds: Rect,
    scale: f32,
}

impl CpuPainter<'_> {
    pub(super) fn projected_layer(&mut self, scope: LayerScope, children: &[PlanNode<'_>]) {
        let Some(frame) = ProjectedFrame::new(&self.state, &scope, self.area(), self.scale) else {
            return;
        };
        let bounds = frame.bounds;
        let width = (bounds.width * self.scale).ceil().max(1.0) as u32;
        let height = (bounds.height * self.scale).ceil().max(1.0) as u32;
        let Some(mut layer) = self.take_pixmap(width, height) else {
            return;
        };
        self.spawn(
            &mut layer,
            Point::new(bounds.origin.x * self.scale, bounds.origin.y * self.scale),
            SceneState::new(Some(bounds), frame.content, 1.0),
        )
        .walk(children);
        if let Some(inverse) = invert(frame.composite.homography())
            && let Some(screen) = self.screen_span(frame)
        {
            let source = Source {
                pixels: layer.data(),
                width: width as usize,
                height: height as usize,
                bounds,
                scale: self.scale,
            };
            self.place(&source, inverse, screen, &scope);
            self.stats.offscreen_layers += 1;
        }
        self.release_pixmap(layer);
    }

    fn place(&mut self, source: &Source<'_>, inverse: Plane, span: [u32; 4], scope: &LayerScope) {
        let view = (self.scale, self.origin);
        if scope.blend == BlendMode::Normal {
            let canvas = Canvas {
                stride: self.target.width() as usize * 4,
                data: self.target.data_mut(),
                at: (0, 0),
            };
            let opacity = scope.opacity * self.state.current_opacity();
            project(canvas, source, (inverse, opacity), span, view);
            return;
        }
        let [x0, y0, x1, y1] = span;
        let Some(mut scratch) = self.take_pixmap(x1 - x0, y1 - y0) else {
            return;
        };
        let canvas = Canvas {
            stride: scratch.width() as usize * 4,
            data: scratch.data_mut(),
            at: (x0, y0),
        };
        project(canvas, source, (inverse, 1.0), span, view);
        self.blended(&mut scratch, (x0, y0), scope);
        self.release_pixmap(scratch);
    }

    fn screen_span(&self, frame: ProjectedFrame) -> Option<[u32; 4]> {
        let mut visible = frame.composite.bounds(frame.bounds)?.intersection(self.area())?;
        if let Some(clip) = self.state.current_clip() {
            visible = visible.intersection(clip)?;
        }
        let (s, o) = (self.scale, self.origin);
        let x0 = visible.origin.x.mul_add(s, -o.x).floor().max(0.0) as u32;
        let y0 = visible.origin.y.mul_add(s, -o.y).floor().max(0.0) as u32;
        let x1 = (visible.origin.x + visible.width).mul_add(s, -o.x).ceil() as u32;
        let y1 = (visible.origin.y + visible.height).mul_add(s, -o.y).ceil() as u32;
        let (x1, y1) = (x1.min(self.target.width()), y1.min(self.target.height()));
        (x0 < x1 && y0 < y1).then_some([x0, y0, x1, y1])
    }
}

struct Canvas<'a> {
    data: &'a mut [u8],
    stride: usize,
    at: (u32, u32),
}

fn project(
    canvas: Canvas<'_>,
    source: &Source<'_>,
    (inverse, opacity): (Plane, f32),
    [x0, y0, x1, y1]: [u32; 4],
    (scale, origin): (f32, Point),
) {
    for py in y0..y1 {
        for px in x0..x1 {
            let at = Point::new(
                (px as f32 + 0.5 + origin.x) / scale,
                (py as f32 + 0.5 + origin.y) / scale,
            );
            let Some(local) = unproject(inverse, at) else {
                continue;
            };
            let amount = opacity * source.coverage(inverse, at, local);
            if amount <= 0.0 {
                continue;
            }
            let texel = source.sample(local);
            let index = (py - canvas.at.1) as usize * canvas.stride + (px - canvas.at.0) as usize * 4;
            if let Some(pixel) = canvas.data.get_mut(index..index + 4) {
                blend(pixel, texel, amount);
            }
        }
    }
}

impl Source<'_> {
    fn edge_distance(&self, local: Point) -> f32 {
        let half = (self.bounds.width * 0.5, self.bounds.height * 0.5);
        let dx = (local.x - self.bounds.origin.x - half.0).abs() - half.0;
        let dy = (local.y - self.bounds.origin.y - half.1).abs() - half.1;
        dx.max(dy)
    }

    fn coverage(&self, inverse: Plane, at: Point, local: Point) -> f32 {
        let step = 1.0 / self.scale;
        let here = self.edge_distance(local);
        let near = |point: Point| unproject(inverse, point).map_or(here, |p| self.edge_distance(p));
        let right = near(Point::new(at.x + step, at.y));
        let below = near(Point::new(at.x, at.y + step));
        let aa = ((right - here).abs() + (below - here).abs()).mul_add(0.5, 1.0e-4);
        1.0 - smoothstep(-aa, aa, here)
    }

    fn texel(&self, x: usize, y: usize) -> [f32; 4] {
        let index = (y.min(self.height - 1) * self.width + x.min(self.width - 1)) * 4;
        self.pixels
            .get(index..index + 4)
            .and_then(|p| <[u8; 4]>::try_from(p).ok())
            .map_or([0.0; 4], |p| p.map(f32::from))
    }

    fn sample(&self, local: Point) -> [f32; 4] {
        let x = (local.x - self.bounds.origin.x).mul_add(self.scale, -0.5).max(0.0);
        let y = (local.y - self.bounds.origin.y).mul_add(self.scale, -0.5).max(0.0);
        let (ix, iy) = (x.floor() as usize, y.floor() as usize);
        let (fx, fy) = (x.fract(), y.fract());
        let top = mix(self.texel(ix, iy), self.texel(ix + 1, iy), fx);
        let bottom = mix(self.texel(ix, iy + 1), self.texel(ix + 1, iy + 1), fx);
        mix(top, bottom, fy)
    }
}

fn unproject(inverse: Plane, at: Point) -> Option<Point> {
    let [u, v, s] = apply(inverse, [at.x, at.y, 1.0]);
    (s > f32::MIN_POSITIVE).then(|| Point::new(u / s, v / s))
}

fn mix(a: [f32; 4], b: [f32; 4], t: f32) -> [f32; 4] {
    let mut out = a;
    for (value, target) in out.iter_mut().zip(b) {
        *value = (target - *value).mul_add(t, *value);
    }
    out
}

fn smoothstep(low: f32, high: f32, value: f32) -> f32 {
    let t = ((value - low) / (high - low)).clamp(0.0, 1.0);
    t * t * 2.0f32.mul_add(-t, 3.0)
}

fn blend(pixel: &mut [u8], texel: [f32; 4], amount: f32) {
    let keep = 1.0 - texel[3] * amount / 255.0;
    for (channel, source) in pixel.iter_mut().zip(texel) {
        let value = source.mul_add(amount, f32::from(*channel) * keep);
        *channel = value.round().clamp(0.0, 255.0) as u8;
    }
}
