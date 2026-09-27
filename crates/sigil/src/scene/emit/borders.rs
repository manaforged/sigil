use crate::hal::{Point, Rect};
use crate::kernel::NodeId;
use crate::scene::builder::SceneBuilder;
use crate::scene::command::{CellRect, SceneRectStyle, Stroke, StrokeAlign, StrokeStyle};
use crate::scene::corner_radii::CornerRadii;
use crate::style::{RenderStyle, Rgba8, Theme};

use super::paint::NodePaint;

pub(super) fn border_edges(style: &RenderStyle) -> [f32; 4] {
    let uniform = style.border_width.map_or(0.0, |px| f32::from(px.0));
    [
        style.border_top.map_or(uniform, |px| f32::from(px.0)),
        style.border_right.map_or(uniform, |px| f32::from(px.0)),
        style.border_bottom.map_or(uniform, |px| f32::from(px.0)),
        style.border_left.map_or(uniform, |px| f32::from(px.0)),
    ]
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
    let theme = paint.theme;
    let [top, right, bottom, left] = border_edges(style);
    if top <= 0.0 && right <= 0.0 && bottom <= 0.0 && left <= 0.0 {
        return;
    }
    if top > 0.0 && borders_uniform([top, right, bottom, left]) {
        return;
    }
    let color = border_color(style, theme);
    let fill = SceneRectStyle::new(color, CornerRadii::new(0.0, 0.0, 0.0, 0.0), style.opacity);
    let bottom = bottom.min(rect.height);
    let right = right.min(rect.width);
    let sides = [
        (
            top,
            Rect::new(rect.origin, rect.width, top.min(rect.height)),
        ),
        (
            bottom,
            Rect::new(
                Point::new(rect.origin.x, rect.origin.y + rect.height - bottom),
                rect.width,
                bottom,
            ),
        ),
        (
            left,
            Rect::new(rect.origin, left.min(rect.width), rect.height),
        ),
        (
            right,
            Rect::new(
                Point::new(rect.origin.x + rect.width - right, rect.origin.y),
                right,
                rect.height,
            ),
        ),
    ];
    let mut cell = CellRect {
        border: paint.border(rect),
        outline: true,
        ..paint.cell()
    };
    for (width, side) in sides {
        if width > 0.0 {
            scene.draw_rect_for_node(id, side, fill, cell);
            cell.border = None;
        }
    }
}
