use std::f32::consts::FRAC_PI_2;
use std::sync::Arc;

use smallvec::{SmallVec, smallvec};

use crate::geometry::{Point, Rect};
use crate::kernel::NodeId;
use crate::scene::builder::SceneBuilder;
use crate::scene::command::{
    DrawPath, PathCmd, PathPaint, PathStroke, StrokeCap, StrokeJoin, outline_key, rounded_outline,
};
use crate::scene::style_adapter::corner_radii_for;
use crate::style::Color;
use crate::style::atom::BorderStyle;

use super::borders::{border_color, border_edges, borders_uniform};
use super::paint::NodePaint;

const IDENTITY: [[f32; 3]; 2] = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
const THIN: f32 = 3.0;
const DOT: f32 = 0.01;

struct Line {
    color: Color,
    opacity: u16,
    style: BorderStyle,
    node: NodeId,
}

pub(super) fn push_styled_border(
    scene: &mut SceneBuilder,
    id: NodeId,
    rect: Rect,
    paint: &NodePaint<'_>,
) -> bool {
    let style = paint.style;
    if style.paint.border == BorderStyle::Solid {
        return false;
    }
    let edges = border_edges(style).map(|width| width.max(0.0));
    let line = Line {
        color: border_color(style, paint.theme),
        opacity: style.opacity,
        style: style.paint.border,
        node: id,
    };
    if borders_uniform(edges) {
        let radii = corner_radii_for(style.border_radius).0;
        push_ring(scene, rect, radii, edges[0], &line);
    } else {
        push_sides(scene, rect, edges, &line);
    }
    true
}

fn push_ring(scene: &mut SceneBuilder, rect: Rect, radii: [f32; 4], width: f32, line: &Line) {
    if width <= 0.0 {
        return;
    }
    let half = width * 0.5;
    let inset = Rect::new(
        Point::new(rect.origin.x + half, rect.origin.y + half),
        (rect.width - width).max(0.0),
        (rect.height - width).max(0.0),
    );
    let limit = inset.width.min(inset.height) * 0.5;
    let radii = radii.map(|radius| (radius - half).clamp(0.0, limit));
    let straight = 2.0 * (inset.width + inset.height);
    let length = radii.iter().fold(straight, |sum, radius| {
        (FRAC_PI_2 - 2.0).mul_add(*radius, sum)
    });
    scene.draw_path(stroked(rounded_outline(inset, radii), width, length, line));
}

fn push_sides(scene: &mut SceneBuilder, rect: Rect, edges: [f32; 4], line: &Line) {
    let (l, t) = (rect.origin.x, rect.origin.y);
    let (r, b) = (l + rect.width, t + rect.height);
    let [top, right, bottom, left] = edges;
    let sides = [
        (top, (l, top.mul_add(0.5, t)), (r, top.mul_add(0.5, t))),
        (
            right,
            (right.mul_add(-0.5, r), t),
            (right.mul_add(-0.5, r), b),
        ),
        (
            bottom,
            (r, bottom.mul_add(-0.5, b)),
            (l, bottom.mul_add(-0.5, b)),
        ),
        (left, (left.mul_add(0.5, l), b), (left.mul_add(0.5, l), t)),
    ];
    for (width, from, to) in sides {
        if width > 0.0 {
            let outline = vec![PathCmd::MoveTo(from.0, from.1), PathCmd::LineTo(to.0, to.1)];
            let length = (to.0 - from.0).hypot(to.1 - from.1);
            scene.draw_path(stroked(outline, width, length, line));
        }
    }
}

fn stroked(outline: Vec<PathCmd>, width: f32, length: f32, line: &Line) -> DrawPath {
    let (dash, cap) = pattern(line.style, width, length);
    let key = outline_key(seed(width, line), &outline);
    let stroke = PathStroke {
        paint: PathPaint::Solid(line.color),
        width,
        cap,
        join: StrokeJoin::Miter,
        dash,
    };
    let mut draw = DrawPath::new(Arc::from(outline), IDENTITY, key).with_stroke(stroke);
    draw.node = Some(line.node);
    draw.opacity = line.opacity;
    draw
}

fn pattern(style: BorderStyle, width: f32, length: f32) -> (SmallVec<[f32; 4]>, StrokeCap) {
    let round = style == BorderStyle::Dotted && width > THIN;
    let (on, off) = match style {
        BorderStyle::Dotted if round => (DOT, 2.0f32.mul_add(width, -DOT)),
        BorderStyle::Dotted => (width, width),
        BorderStyle::Dashed if width < THIN => (width * 3.0, width * 2.0),
        BorderStyle::Dashed | BorderStyle::Solid => (width * 2.0, width),
    };
    let period = on + off;
    let count = (length / period).round().max(1.0);
    let fit = if length > 0.0 {
        length / (count * period)
    } else {
        1.0
    };
    let cap = if round {
        StrokeCap::Round
    } else {
        StrokeCap::Butt
    };
    let on = if round { on } else { on * fit };
    (smallvec![on, period.mul_add(fit, -on)], cap)
}

fn seed(width: f32, line: &Line) -> u64 {
    let [r, g, b, a] = [line.color.r, line.color.g, line.color.b, line.color.a];
    let color = u64::from(u32::from_le_bytes([r, g, b, a]));
    let style = match line.style {
        BorderStyle::Solid => 0,
        BorderStyle::Dashed => 1,
        BorderStyle::Dotted => 2,
    };
    crate::fnv::mix(crate::fnv::mix(color, style), u64::from(width.to_bits()))
}
