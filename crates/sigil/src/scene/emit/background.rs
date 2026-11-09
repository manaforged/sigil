#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "image sides and tile counts are small and checked against MAX_TILES"
)]

use std::f32::consts::SQRT_2;

use crate::geometry::{Point, Rect};
use crate::kernel::{Node, NodeId, ObjectFit};
use crate::scene::builder::SceneBuilder;
use crate::scene::command::fills::TRANSPARENT;
use crate::scene::command::{DrawImage, Gradient, GradientKind, GradientStop};
use crate::style::atom::{GradientShape, PaintStyle};
use crate::style::image_paint::lookup;
use crate::style::{
    BgGradient, Conic, GradientAxis, Radial, RadialExtent, RenderStyle, StyleEngine, Theme,
};

const MAX_TILES: usize = 4096;
const DEFAULT_POSITIONS: [u8; 3] = [0, 50, 100];

pub(super) fn fill(rect: Rect, style: &RenderStyle, theme: &Theme) -> Option<Gradient> {
    if let Some(spec) = style.background_gradient {
        return Some(legacy(rect, spec, theme));
    }
    let shape = style.paint.gradient?;
    let stops = tailwind_stops(&style.paint);
    let (kind, repeat) = match shape {
        GradientShape::Linear(axis) => (linear(rect, axis), false),
        GradientShape::Radial(radial) => (self::radial(rect, radial), radial.repeating),
        GradientShape::Conic(conic) => (self::conic(rect, conic), conic.repeating),
    };
    Some(Gradient::new(kind, stops).repeat(repeat))
}

fn legacy(rect: Rect, spec: BgGradient, theme: &Theme) -> Gradient {
    let engine = StyleEngine::new(theme);
    let stops: Vec<GradientStop> = spec
        .stops
        .iter()
        .map(|stop| {
            GradientStop::new(
                f32::from(stop.offset) / 1000.0,
                engine.resolve_color(stop.color),
            )
        })
        .collect();
    Gradient::new(linear(rect, spec.axis), stops)
}

fn tailwind_stops(paint: &PaintStyle) -> Vec<GradientStop> {
    let stop = |index: usize| {
        let at = paint.positions[index].unwrap_or(DEFAULT_POSITIONS[index]);
        let color = paint.stops[index].unwrap_or(TRANSPARENT);
        GradientStop::new(f32::from(at) / 100.0, color)
    };
    if paint.stops[1].is_some() {
        vec![stop(0), stop(1), stop(2)]
    } else {
        vec![stop(0), stop(2)]
    }
}

fn linear(rect: Rect, axis: GradientAxis) -> GradientKind {
    let (left, top) = (rect.origin.x, rect.origin.y);
    let (right, bottom) = (left + rect.width, top + rect.height);
    let (mid_x, mid_y) = (left + rect.width / 2.0, top + rect.height / 2.0);
    let (start, end) = match axis {
        GradientAxis::TopToBottom => ((mid_x, top), (mid_x, bottom)),
        GradientAxis::BottomToTop => ((mid_x, bottom), (mid_x, top)),
        GradientAxis::LeftToRight => ((left, mid_y), (right, mid_y)),
        GradientAxis::RightToLeft => ((right, mid_y), (left, mid_y)),
    };
    GradientKind::Linear {
        start: Point::new(start.0, start.1),
        end: Point::new(end.0, end.1),
    }
}

fn center_of(rect: Rect, [x, y]: [u8; 2]) -> Point {
    Point::new(
        rect.width.mul_add(f32::from(x) / 100.0, rect.origin.x),
        rect.height.mul_add(f32::from(y) / 100.0, rect.origin.y),
    )
}

pub(super) fn radial(rect: Rect, radial: Radial) -> GradientKind {
    let center = center_of(rect, radial.at);
    let dx = [
        center.x - rect.origin.x,
        rect.origin.x + rect.width - center.x,
    ];
    let dy = [
        center.y - rect.origin.y,
        rect.origin.y + rect.height - center.y,
    ];
    let near = [dx[0].min(dx[1]), dy[0].min(dy[1])];
    let far = [dx[0].max(dx[1]), dy[0].max(dy[1])];
    let radius = match (radial.circle, radial.extent) {
        (false, RadialExtent::ClosestSide) => near,
        (false, RadialExtent::FarthestSide) => far,
        (false, RadialExtent::ClosestCorner) => near.map(|side| side * SQRT_2),
        (false, RadialExtent::FarthestCorner) => far.map(|side| side * SQRT_2),
        (true, RadialExtent::ClosestSide) => [near[0].min(near[1]); 2],
        (true, RadialExtent::FarthestSide) => [far[0].max(far[1]); 2],
        (true, RadialExtent::ClosestCorner) => [near[0].hypot(near[1]); 2],
        (true, RadialExtent::FarthestCorner) => [far[0].hypot(far[1]); 2],
    };
    GradientKind::Radial { center, radius }
}

pub(super) fn conic(rect: Rect, conic: Conic) -> GradientKind {
    GradientKind::Conic {
        center: center_of(rect, conic.at),
        angle: f32::from(conic.from),
    }
}

pub(super) fn push_image(
    scene: &mut SceneBuilder,
    node: (NodeId, &Node),
    rect: Rect,
    style: &RenderStyle,
) {
    let (id, node) = node;
    let Some(source) = style.paint.image.and_then(|image| lookup(node, image)) else {
        return;
    };
    let (width, height) = (source.width() as f32, source.height() as f32);
    let tiles = |extent: f32, side: f32| (extent / side).ceil().max(0.0) as usize;
    let (columns, rows) = (tiles(rect.width, width), tiles(rect.height, height));
    if width < 1.0 || height < 1.0 || columns.saturating_mul(rows) > MAX_TILES {
        return;
    }
    scene.with_clip(rect, |scene| {
        for row in 0..rows {
            for column in 0..columns {
                let origin = Point::new(
                    width.mul_add(column as f32, rect.origin.x),
                    height.mul_add(row as f32, rect.origin.y),
                );
                scene.draw_image(DrawImage {
                    node: Some(id),
                    source: source.clone(),
                    fit: ObjectFit::Fill,
                    destination: Rect::new(origin, width, height),
                    opacity: style.opacity,
                    crop: None,
                    raster: None,
                });
            }
        }
    });
}
