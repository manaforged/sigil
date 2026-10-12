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
use crate::renderer::scene_command::state::SceneState;
use crate::renderer::scene_command::walk::{IDENTITY, transform_rect};
use crate::renderer::scene_command::{LayerEffect, LayerScope, SceneCommand, Transform};
use crate::scene::corner_radii::CornerRadii;

use super::bound::rect_mask;
use super::paint::{Painter, fade};
use super::shadow::blur;
use super::shapes::{blit, cover, ts_transform};

impl Painter<'_> {
    fn take_pixmap(&mut self, width: u32, height: u32) -> Option<Pixmap> {
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

    fn release_pixmap(&mut self, layer: Pixmap) {
        if self.pool.len() < CacheLimits::DEFAULT.cpu_layer_pool {
            self.pool.push(layer);
        }
    }

    pub(super) fn layer(&mut self, scope: LayerScope, children: &[SceneCommand]) {
        if let Some(effect) = SceneState::inline_transform(&scope) {
            self.state.push_transform(effect);
            self.state.push_clip(scope.clip);
            if !self.state.clipped_out() {
                self.walk(children);
            }
            self.state.pop_clip();
            self.state.pop_transform();
            return;
        }
        if let LayerEffect::Transform(transform) = scope.effect {
            self.transformed_layer(scope, transform, children);
            return;
        }
        self.untransformed_layer(scope, children);
    }

    fn area(&self) -> Rect {
        Rect::new(
            Point::new(self.origin.x / self.scale, self.origin.y / self.scale),
            self.target.width() as f32 / self.scale,
            self.target.height() as f32 / self.scale,
        )
    }

    pub(super) fn group(&mut self, opacity: f32, children: &[SceneCommand]) {
        let Some(bounds) = self.state.draw_bounds(children, self.area(), self.scale) else {
            return;
        };
        let state = self.state.group_state(bounds);
        let Some((layer, at)) = self.render_region(bounds, state, children) else {
            return;
        };
        self.composite(&layer, at, opacity, None);
        self.release_pixmap(layer);
    }

    fn untransformed_layer(&mut self, scope: LayerScope, children: &[SceneCommand]) {
        let clip_logical = transform_rect(self.state.current_transform(), scope.clip);
        let Some(bounds) = self.state.layer_bounds(&scope, self.area()) else {
            return;
        };
        let state = SceneState::new(Some(bounds), self.state.current_transform(), 1.0);
        let region = SceneState::layer_region(bounds, self.area(), self.scale);
        let Some((mut layer, at)) = self.render_region(region, state, children) else {
            return;
        };
        if let LayerEffect::LayerBlur { sigma } = scope.effect {
            blur(&mut layer, sigma * self.scale);
        }
        let rounded = scope.clip_radii.0.iter().any(|radius| *radius > 0.0);
        let mask = rounded
            .then(|| self.layer_mask(clip_logical, scope.clip_radii))
            .flatten();
        self.composite(&layer, at, scope.opacity, mask.as_ref());
        self.release_pixmap(layer);
    }

    fn composite(
        &mut self,
        layer: &Pixmap,
        transform: TsTransform,
        opacity: f32,
        mask: Option<&Mask>,
    ) {
        let opacity = opacity * self.state.current_opacity();
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
        blit(target, layer, transform, faded);
        stats.offscreen_layers += 1;
    }

    fn render_region(
        &mut self,
        bounds: Rect,
        state: SceneState,
        children: &[SceneCommand],
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
        children: &[SceneCommand],
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
        self.composite(&layer, placement, scope.opacity, mask.as_ref());
        self.release_pixmap(layer);
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
