use crate::geometry::{Point, Rect};
use crate::kernel::NodeId;
use crate::scene::builder::SceneBuilder;
use crate::scene::command::{CellRect, SceneRectStyle, Stroke, StrokeAlign, StrokeStyle};
use crate::scene::corner_radii::CornerRadii;
use crate::style::{RenderStyle, Rgba8, Theme};

use super::paint::NodePaint;

pub(super) fn border_edges(style: &RenderStyle) -> [f32; 4] {
    let uniform = style.border_width.map_or(0.0, |px| f32::from(px.0));
    [
        style.border_top,
        style.border_right,
        style.border_bottom,
        style.border_left,
    ]
    .map(|side| side.map_or(uniform, |px| f32::from(px.0)))
}

pub(super) fn border_color(style: &RenderStyle, theme: &Theme) -> Rgba8 {
    style
        .border_color
        .or(style.text_color)
        .unwrap_or(theme.on_surface)
}

pub(super) const fn borders_uniform([top, right, bottom, left]: [f32; 4]) -> bool {
    top.to_bits() == right.to_bits()
        && right.to_bits() == bottom.to_bits()
        && bottom.to_bits() == left.to_bits()
}

pub(super) fn border_stroke(style: &RenderStyle, theme: &Theme) -> Option<Stroke> {
    let [top, right, bottom, left] = border_edges(style);
    if top <= 0.0 || !borders_uniform([top, right, bottom, left]) {
        return None;
    }
    Some(Stroke {
        color: border_color(style, theme),
        width: top,
        style: StrokeStyle::Solid,
        align: StrokeAlign::Inside,
    })
}

pub(super) fn push_side_borders(
    scene: &mut SceneBuilder,
    id: NodeId,
    rect: Rect,
    paint: &NodePaint<'_>,
) {
    let style = paint.style;
    let edges = border_edges(style);
    let [top, right, bottom, left] = edges;
    if edges.iter().all(|width| *width <= 0.0) || (top > 0.0 && borders_uniform(edges)) {
        return;
    }
    let color = border_color(style, paint.theme);
    let fill = SceneRectStyle::new(color, CornerRadii::ZERO, style.opacity);
    let Rect {
        origin,
        width,
        height,
    } = rect;
    let (bottom, right) = (bottom.min(height), right.min(width));
    let sides = [
        (top, Rect::new(origin, width, top.min(height))),
        (
            bottom,
            Rect::new(
                Point::new(origin.x, origin.y + height - bottom),
                width,
                bottom,
            ),
        ),
        (left, Rect::new(origin, left.min(width), height)),
        (
            right,
            Rect::new(
                Point::new(origin.x + width - right, origin.y),
                right,
                height,
            ),
        ),
    ];
    let mut cell = CellRect {
        border: paint.border(rect),
        outline: true,
        ..paint.cell()
    };
    for (edge, side) in sides {
        if edge > 0.0 {
            scene.draw_rect_for_node(id, side, fill, cell);
            cell.border = None;
        }
    }
}
