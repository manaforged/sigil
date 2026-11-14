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
use crate::style::atom::{Geometry, GradientShape, PaintStyle};
use crate::style::image_paint::lookup;
use crate::style::{Conic, Radial, RadialExtent, RenderStyle};

const MAX_TILES: usize = 4096;

pub(super) fn fill(rect: Rect, style: &RenderStyle) -> Option<Gradient> {
    let shape = style.paint.gradient?;
    let stops = tailwind_stops(&style.paint);
    let driven = style.paint.geometry;
    let (kind, repeat) = match shape {
        GradientShape::Linear(axis) => (linear(rect, driven.direction(axis)), false),
        GradientShape::Radial(radial) => (
            ellipse(rect, radial, driven.center(radial.at)),
            radial.repeating,
        ),
        GradientShape::Conic(conic) => (
            sweep(
                rect,
                driven.center(conic.at),
                driven.angle(f32::from(conic.from)),
            ),
            conic.repeating,
        ),
    };
    Some(Gradient::new(kind, stops).repeat(repeat))
}

fn tailwind_stops(paint: &PaintStyle) -> Vec<GradientStop> {
    let [first, middle, last] = std::array::from_fn(|index| {
        let color = paint.stops.get(index).copied().flatten();
        GradientStop::new(
            f32::from(paint.at(index)) / 100.0,
            color.unwrap_or(TRANSPARENT),
        )
    });
    let [_, via, _] = paint.stops;
    if via.is_some() {
        vec![first, middle, last]
    } else {
        vec![first, last]
    }
}

pub(super) fn linear(rect: Rect, [dx, dy]: [f32; 2]) -> GradientKind {
    let half = (rect.width * dx)
        .abs()
        .mul_add(0.5, (rect.height * dy).abs() * 0.5);
    let center = center_of(rect, [50.0, 50.0]);
    GradientKind::Linear {
        start: Point::new(dx.mul_add(-half, center.x), dy.mul_add(-half, center.y)),
        end: Point::new(dx.mul_add(half, center.x), dy.mul_add(half, center.y)),
    }
}

fn center_of(rect: Rect, [x, y]: [f32; 2]) -> Point {
    Point::new(
        rect.width.mul_add(x / 100.0, rect.origin.x),
        rect.height.mul_add(y / 100.0, rect.origin.y),
    )
}

pub(super) fn radial(rect: Rect, radial: Radial) -> GradientKind {
    ellipse(rect, radial, Geometry::default().center(radial.at))
}

fn ellipse(rect: Rect, radial: Radial, at: [f32; 2]) -> GradientKind {
    let center = center_of(rect, at);
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
    sweep(
        rect,
        Geometry::default().center(conic.at),
        f32::from(conic.from),
    )
}

fn sweep(rect: Rect, at: [f32; 2], angle: f32) -> GradientKind {
    GradientKind::Conic {
        center: center_of(rect, at),
        angle,
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
    scene.clip(rect, |scene| {
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
