#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    reason = "pixel coordinates are bounded by the target size; float-to-int rounding is the rasterizer contract"
)]

use crate::geometry::{Point, Rect};
use tiny_skia::{Mask, Pixmap, Transform as TsTransform};

use crate::renderer::cache_limits::CacheLimits;
use crate::renderer::plan::{MaskSource, PlanNode};
use crate::renderer::scene_command::state::SceneState;
use crate::renderer::scene_command::walk::{IDENTITY, transform_rect};
use crate::renderer::scene_command::{LayerEffect, LayerScope, SceneCommand, Transform};
use crate::scene::command::BlendMode;
use crate::scene::corner_radii::CornerRadii;

use super::bound::rect_mask;
use super::clip::multiplied;
use super::paint::{CpuPainter, fade};
use super::shapes::{blit, cover, ts_transform};

impl CpuPainter<'_> {
    pub(super) fn take_pixmap(&mut self, width: u32, height: u32) -> Option<Pixmap> {
        if let Some(at) = self
            .pool
            .iter()
            .position(|p| p.width() == width && p.height() == height)
        {
            let mut reused = self.pool.swap_remove(at);
            reused.fill(tiny_skia::Color::TRANSPARENT);
            return Some(reused);
        }
        Pixmap::new(width, height)
    }

    pub(super) fn release_pixmap(&mut self, layer: Pixmap) {
        if self.pool.len() < CacheLimits::DEFAULT.cpu_layer_pool {
            self.pool.push(layer);
        }
    }

    pub(super) fn layer(
        &mut self,
        scope: LayerScope,
        mask: Option<MaskSource<'_>>,
        children: &[PlanNode<'_>],
    ) {
        if let LayerEffect::Transform(transform) = scope.effect {
            self.transformed_layer(scope, transform, children);
            return;
        }
        if let LayerEffect::Projected { .. } = scope.effect {
            self.projected_layer(scope, children);
            return;
        }
        self.untransformed_layer(scope, mask, children);
    }

    pub(super) fn area(&self) -> Rect {
        Rect::new(
            Point::new(self.origin.x / self.scale, self.origin.y / self.scale),
            self.target.width() as f32 / self.scale,
            self.target.height() as f32 / self.scale,
        )
    }

    pub(super) fn group(
        &mut self,
        opacity: f32,
        commands: &[SceneCommand],
        children: &[PlanNode<'_>],
    ) {
        let Some(bounds) = self.state.draw_bounds(commands, self.area(), self.scale) else {
            return;
        };
        let state = self.state.group_state(bounds);
        let Some((layer, at)) = self.render_region(bounds, state, children) else {
            return;
        };
        self.composite(&layer, at, Ink::plain(opacity), None);
        self.release_pixmap(layer);
    }

    fn untransformed_layer(
        &mut self,
        scope: LayerScope,
        mask: Option<MaskSource<'_>>,
        children: &[PlanNode<'_>],
    ) {
        let clip_logical = transform_rect(self.state.current_transform(), scope.clip);
        let Some(bounds) = self.state.layer_bounds(&scope, self.area()) else {
            return;
        };
        let state = SceneState::new(Some(bounds), self.state.current_transform(), 1.0);
        let region = SceneState::layer_region(bounds, self.area(), self.scale);
        let Some((mut layer, at)) = self.render_region(region, state, children) else {
            return;
        };
        super::filter::filter(&mut layer, &scope.effect, self.scale);
        let rounded = self.rounded_mask(clip_logical, scope.clip_radii);
        let keep = match mask {
            Some(MaskSource::Path(draw)) => self.path_mask(draw, bounds).map(Some),
            Some(MaskSource::Scene(mask)) => self
                .scene_mask(mask, region, bounds)
                .map(|keep| Some(multiplied(keep, rounded.as_ref()))),
            None => Some(rounded),
        };
        if let Some(mask) = keep {
            let ink = Ink::of(&scope).prepared(&mut layer);
            self.composite(&layer, at, ink, mask.as_ref());
        }
        self.release_pixmap(layer);
    }

    fn composite(
        &mut self,
        layer: &Pixmap,
        transform: TsTransform,
        ink: Ink,
        mask: Option<&Mask>,
    ) {
        let opacity = ink.opacity * self.state.current_opacity();
        let (width, height) = (self.target.width(), self.target.height());
        let Self {
            target,
            mask: masks,
            stats,
            ..
        } = self;
        let Some(base) = mask.or_else(|| cover(masks.covers, width, height)) else {
            return;
        };
        let Some(faded) = fade(&mut masks.faded, base, opacity) else {
            return;
        };
        blit(target, layer, transform, faded, ink.blend);
        stats.offscreen_layers += 1;
    }

    pub(super) fn render_region(
        &mut self,
        bounds: Rect,
        state: SceneState,
        children: &[PlanNode<'_>],
    ) -> Option<(Pixmap, TsTransform)> {
        let x0 = bounds.origin.x.mul_add(self.scale, -self.origin.x).round();
        let y0 = bounds.origin.y.mul_add(self.scale, -self.origin.y).round();
        let x1 = (bounds.origin.x + bounds.width)
            .mul_add(self.scale, -self.origin.x)
            .round();
        let y1 = (bounds.origin.y + bounds.height)
            .mul_add(self.scale, -self.origin.y)
            .round();
        let mut layer = self.take_pixmap((x1 - x0).max(1.0) as u32, (y1 - y0).max(1.0) as u32)?;
        self.spawn(
            &mut layer,
            Point::new(self.origin.x + x0, self.origin.y + y0),
            state,
        )
        .walk(children);
        let at = TsTransform::from_translate(x0 as i32 as f32, y0 as i32 as f32);
        Some((layer, at))
    }

    fn transformed_layer(
        &mut self,
        scope: LayerScope,
        transform: Transform,
        children: &[PlanNode<'_>],
    ) {
        let Some(frame) = self
            .state
            .transform_frame(&scope, transform, self.area(), self.scale)
        else {
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
        if let Some(mask) = source_mask(frame.scope, bounds, (width, height), self.scale) {
            layer.apply_mask(&mask);
        }
        let placement = TsTransform::from_scale(1.0 / self.scale, 1.0 / self.scale)
            .post_translate(bounds.origin.x, bounds.origin.y)
            .post_concat(ts_transform(frame.composite, self.scale, self.origin));
        let mask = self
            .state
            .current_clip()
            .and_then(|clip| self.layer_mask(clip, CornerRadii::ZERO));
        let ink = Ink::of(&scope).prepared(&mut layer);
        self.composite(&layer, placement, ink, mask.as_ref());
        self.release_pixmap(layer);
    }

    fn rounded_mask(&self, clip: Rect, radii: CornerRadii) -> Option<Mask> {
        let rounded = radii.0.iter().any(|radius| *radius > 0.0);
        rounded.then(|| self.layer_mask(clip, radii)).flatten()
    }

    fn layer_mask(&self, clip: Rect, radii: CornerRadii) -> Option<Mask> {
        let transform = ts_transform(IDENTITY, self.scale, self.origin);
        rect_mask(
            clip,
            radii,
            transform,
            (self.target.width(), self.target.height()),
        )
    }
}

fn source_mask(scope: LayerScope, bounds: Rect, size: (u32, u32), scale: f32) -> Option<Mask> {
    if scope.clip_radii.0.iter().all(|radius| *radius <= 0.0) {
        return None;
    }
    let origin = Point::new(bounds.origin.x * scale, bounds.origin.y * scale);
    let transform = ts_transform(IDENTITY, scale, origin);
    rect_mask(scope.clip, scope.clip_radii, transform, size)
}

#[derive(Clone, Copy)]
struct Ink {
    opacity: f32,
    blend: tiny_skia::BlendMode,
}

impl Ink {
    const fn plain(opacity: f32) -> Self {
        Self {
            opacity,
            blend: tiny_skia::BlendMode::SourceOver,
        }
    }

    const fn of(scope: &LayerScope) -> Self {
        Self {
            opacity: scope.opacity,
            blend: skia_blend(scope.blend),
        }
    }

    fn prepared(self, layer: &mut Pixmap) -> Self {
        if !matches!(self.blend, tiny_skia::BlendMode::SourceOver) {
            encode_premultiplied(layer);
        }
        self
    }
}

fn encode_premultiplied(layer: &mut Pixmap) {
    for pixel in layer.data_mut().chunks_exact_mut(4) {
        let [r, g, b, a] = [0, 1, 2, 3].map(|i| pixel.get(i).copied().unwrap_or(0));
        if a == 0 || a == u8::MAX {
            continue;
        }
        let gain = (f32::from(a) / 255.0).sqrt();
        let scaled = |channel: u8| ((f32::from(channel) * gain).round() as u8).min(a);
        pixel.copy_from_slice(&[scaled(r), scaled(g), scaled(b), a]);
    }
}

const fn skia_blend(mode: BlendMode) -> tiny_skia::BlendMode {
    use tiny_skia::BlendMode as Skia;
    match mode {
        BlendMode::Normal => Skia::SourceOver,
        BlendMode::Multiply => Skia::Multiply,
        BlendMode::Screen => Skia::Screen,
        BlendMode::Overlay => Skia::Overlay,
        BlendMode::Darken => Skia::Darken,
        BlendMode::Lighten => Skia::Lighten,
        BlendMode::ColorDodge => Skia::ColorDodge,
        BlendMode::ColorBurn => Skia::ColorBurn,
        BlendMode::HardLight => Skia::HardLight,
        BlendMode::SoftLight => Skia::SoftLight,
        BlendMode::Difference => Skia::Difference,
        BlendMode::Exclusion => Skia::Exclusion,
        BlendMode::Hue => Skia::Hue,
        BlendMode::Saturation => Skia::Saturation,
        BlendMode::Color => Skia::Color,
        BlendMode::Luminosity => Skia::Luminosity,
    }
}
