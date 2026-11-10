#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    reason = "image sides and tile counts are small and checked against MAX_TILES"
)]

use crate::geometry::{Point, Rect};
use crate::kernel::{ImageSource, Node, NodeId, ObjectFit};
use crate::layout::LayoutResult;
use crate::scene::Clip;
use crate::scene::builder::SceneBuilder;
use crate::scene::command::fills::TRANSPARENT;
use crate::scene::command::{
    BlendMode, DrawGradient, DrawImage, Gradient, GradientKind, GradientStop, LayerEffect,
    LayerScope, Mask, MaskMode,
};
use crate::scene::corner_radii::CornerRadii;
use crate::style::atom::{MaskKind, MaskStyle};
use crate::style::image_paint::lookup;
use crate::style::{Color, MaskSize, RenderStyle};

use super::{EmitWalk, Inherited};

const MAX_TILES: i32 = 4096;
const BLACK: Color = Color {
    r: 0,
    g: 0,
    b: 0,
    a: 255,
};

impl<'t> EmitWalk<'_, 't> {
    pub(super) fn push_masked(
        &mut self,
        id: NodeId,
        inherited: &Inherited<'t>,
        clip: Option<Rect>,
        mut style: RenderStyle,
    ) {
        let node = self.cx.get(id);
        let Some(rect) = node
            .and_then(|node| node.get_extension::<LayoutResult>())
            .map(LayoutResult::rect)
        else {
            self.push_filtered(id, inherited, clip, &style);
            return;
        };
        if let Some(shape) = style.clip_path.take() {
            self.nested(LayerOrClip::Clip(shape.resolve(rect)), |walk| {
                walk.push_masked(id, inherited, clip, style);
            });
            return;
        }
        let Some(mask) = node.and_then(|node| mask_for(&style.mask, rect, node)) else {
            self.push_filtered(id, inherited, clip, &style);
            return;
        };
        style.mask = MaskStyle::NONE;
        let scope = LayerScope {
            clip: rect,
            clip_radii: CornerRadii::ZERO,
            effect: LayerEffect::Mask(mask),
            opacity: 1.0,
            blend: BlendMode::Normal,
        };
        self.nested(LayerOrClip::Layer(scope), |walk| {
            walk.push_filtered(id, inherited, clip, &style);
        });
    }

    pub(super) fn nested(&mut self, around: LayerOrClip, body: impl FnOnce(&mut EmitWalk<'_, 't>)) {
        let (cx, deferred, selection) = (self.cx, &mut *self.deferred, self.selection);
        let track = &mut *self.track;
        let inner = |scene: &mut SceneBuilder| {
            body(&mut EmitWalk {
                scene,
                cx,
                deferred,
                selection,
                track,
            });
        };
        match around {
            LayerOrClip::Layer(scope) => self.scene.with_composite_layer(scope, inner),
            LayerOrClip::Clip(shape) => self.scene.with_clip(shape, inner),
        }
    }
}

pub(super) enum LayerOrClip {
    Layer(LayerScope),
    Clip(Clip),
}

fn mask_for(mask: &MaskStyle, rect: Rect, node: &Node) -> Option<Mask> {
    let mut scene = SceneBuilder::new();
    match mask.kind? {
        MaskKind::Image => draw_tiles(&mut scene, mask, rect, lookup(node, mask.image)?),
        kind => scene.draw_gradient(DrawGradient::new(rect, gradient(mask, kind, rect))),
    }
    let mode = if mask.luminance {
        MaskMode::Luminance
    } else {
        MaskMode::Alpha
    };
    Some(Mask::new(mode, scene.finish()))
}

fn gradient(mask: &MaskStyle, kind: MaskKind, rect: Rect) -> Gradient {
    let [from, to] = mask.stops.map(|percent| f32::from(percent) / 100.0);
    let stops = vec![
        GradientStop::new(from, BLACK),
        GradientStop::new(to, TRANSPARENT),
    ];
    let (kind, repeat) = match kind {
        MaskKind::Radial => (
            super::background::radial(rect, mask.radial),
            mask.radial.repeating,
        ),
        MaskKind::Conic => (
            super::background::conic(rect, mask.conic),
            mask.conic.repeating,
        ),
        MaskKind::Linear | MaskKind::Image => (linear(rect, mask.angle), false),
    };
    Gradient::new(kind, stops).repeat(repeat)
}

fn linear(rect: Rect, degrees: i16) -> GradientKind {
    let (sin, cos) = f32::from(degrees).to_radians().sin_cos();
    let half = (rect.width * sin)
        .abs()
        .mul_add(0.5, (rect.height * cos).abs() * 0.5);
    let center = Point::new(
        rect.width.mul_add(0.5, rect.origin.x),
        rect.height.mul_add(0.5, rect.origin.y),
    );
    GradientKind::Linear {
        start: Point::new(sin.mul_add(-half, center.x), cos.mul_add(half, center.y)),
        end: Point::new(sin.mul_add(half, center.x), cos.mul_add(-half, center.y)),
    }
}

fn tile_size(size: MaskSize, rect: Rect, image: (f32, f32)) -> (f32, f32) {
    let (fit_x, fit_y) = (rect.width / image.0, rect.height / image.1);
    let scale = match size {
        MaskSize::Auto => 1.0,
        MaskSize::Cover => fit_x.max(fit_y),
        MaskSize::Contain => fit_x.min(fit_y),
    };
    (image.0 * scale, image.1 * scale)
}

fn draw_tiles(scene: &mut SceneBuilder, mask: &MaskStyle, rect: Rect, source: &ImageSource) {
    let natural = (source.width() as f32, source.height() as f32);
    if natural.0 < 1.0 || natural.1 < 1.0 {
        return;
    }
    let (width, height) = tile_size(mask.size, rect, natural);
    let [px, py] = mask.position.map(|units| f32::from(units) / 10_000.0);
    let origin = Point::new(
        (rect.width - width).mul_add(px, rect.origin.x),
        (rect.height - height).mul_add(py, rect.origin.y),
    );
    let span = |start: f32, low: f32, extent: f32, side: f32| {
        if !mask.repeat || side < 0.5 {
            return 0..=0;
        }
        let before = ((start - low) / side).ceil() as i32;
        let after = ((low + extent - start) / side).ceil() as i32;
        -before.min(MAX_TILES)..=after.min(MAX_TILES)
    };
    let columns = span(origin.x, rect.origin.x, rect.width, width);
    let rows = span(origin.y, rect.origin.y, rect.height, height);
    if (columns.end() - columns.start() + 1) * (rows.end() - rows.start() + 1) > MAX_TILES {
        return;
    }
    for row in rows {
        for column in columns.clone() {
            let at = Point::new(
                width.mul_add(column as f32, origin.x),
                height.mul_add(row as f32, origin.y),
            );
            scene.draw_image(DrawImage {
                node: None,
                source: source.clone(),
                fit: ObjectFit::Fill,
                destination: Rect::new(at, width, height),
                opacity: 10_000,
                crop: None,
                raster: None,
            });
        }
    }
}
