use std::sync::Arc;

use smallvec::SmallVec;

use super::key::{hash_cmd, mix, mix_f32, mix_fill, mix_stroke};
use super::{PathCmd, PathFill, PathNode};
use crate::fnv::OFFSET;
use crate::geometry::Point;
use crate::scene::command::{GradientStop, LinearGradient, PathPaint, Transform};
use crate::style::{Rgba8, Theme};

type Resolved = (Option<PathPaint>, Option<(Rgba8, f32)>);

pub(super) fn resolve(
    fill: Option<&PathFill>,
    stroke: Option<(super::PathColor, f32)>,
    theme: &Theme,
) -> Resolved {
    let point = |(x, y): (f32, f32)| Point::new(x, y);
    let stops = |stops: &[(f32, Rgba8)]| -> SmallVec<[GradientStop; 4]> {
        stops
            .iter()
            .map(|&(offset, color)| GradientStop::new(offset, color))
            .collect()
    };
    let fill = fill.map(|fill| match fill {
        PathFill::Solid(color) => PathPaint::Solid(*color),
        PathFill::Theme(token) => PathPaint::Solid(theme.color(*token)),
        PathFill::Linear { start, end, stops: list } => {
            PathPaint::Linear(LinearGradient::new(point(*start), point(*end), stops(list)))
        }
        PathFill::Radial {
            center,
            radius,
            stops: list,
        } => PathPaint::Radial {
            center: point(*center),
            radius: *radius,
            stops: stops(list),
        },
    });
    let stroke = stroke.map(|(color, width)| (color.resolve(theme), width));
    (fill, stroke)
}

pub(super) fn path_key(node: &PathNode, fill: Option<&PathPaint>, stroke: Option<(Rgba8, f32)>) -> u64 {
    let mut hash = OFFSET;
    for byte in node.width.to_le_bytes().into_iter().chain(node.height.to_le_bytes()) {
        mix(&mut hash, byte);
    }
    mix(&mut hash, u8::from(node.non_scaling));
    mix(&mut hash, node.rule as u8);
    mix(&mut hash, node.cap.map_or(0, |cap| cap as u8 + 1));
    mix(&mut hash, node.join.map_or(0, |join| join as u8 + 1));
    for length in &node.dash {
        mix_f32(&mut hash, *length);
    }
    for cmd in &node.cmds {
        hash_cmd(&mut hash, *cmd);
    }
    mix_stroke(&mut hash, stroke);
    mix_fill(&mut hash, fill);
    hash
}

pub(super) fn theme_key(node: &PathNode, theme: &Theme) -> u64 {
    let (fill, stroke) = resolve(node.fill.as_ref(), node.stroke, theme);
    let mut hash = OFFSET;
    mix_stroke(&mut hash, stroke);
    mix_fill(&mut hash, fill.as_ref());
    hash
}

fn apply([[sx, kx, tx], [ky, sy, ty]]: Transform, (x, y): (f32, f32)) -> (f32, f32) {
    (sx.mul_add(x, kx.mul_add(y, tx)), ky.mul_add(x, sy.mul_add(y, ty)))
}

pub(super) fn bake(cmds: &[PathCmd], view: Transform) -> Arc<[PathCmd]> {
    let at = |x, y| apply(view, (x, y));
    cmds.iter()
        .map(|cmd| match *cmd {
            PathCmd::MoveTo(x, y) => {
                let (x, y) = at(x, y);
                PathCmd::MoveTo(x, y)
            }
            PathCmd::LineTo(x, y) => {
                let (x, y) = at(x, y);
                PathCmd::LineTo(x, y)
            }
            PathCmd::QuadTo(cx, cy, x, y) => {
                let ((cx, cy), (x, y)) = (at(cx, cy), at(x, y));
                PathCmd::QuadTo(cx, cy, x, y)
            }
            PathCmd::CubicTo(c1x, c1y, c2x, c2y, x, y) => {
                let ((c1x, c1y), (c2x, c2y), (x, y)) = (at(c1x, c1y), at(c2x, c2y), at(x, y));
                PathCmd::CubicTo(c1x, c1y, c2x, c2y, x, y)
            }
            PathCmd::Close => PathCmd::Close,
        })
        .collect()
}

pub(super) fn map(paint: &PathPaint, view: Transform) -> PathPaint {
    let at = |p: Point| {
        let (x, y) = apply(view, (p.x, p.y));
        Point::new(x, y)
    };
    match paint {
        PathPaint::Solid(color) => PathPaint::Solid(*color),
        PathPaint::Linear(linear) => {
            PathPaint::Linear(LinearGradient::new(at(linear.start), at(linear.end), linear.stops.clone()))
        }
        PathPaint::Radial {
            center,
            radius,
            stops,
        } => PathPaint::Radial {
            center: at(*center),
            radius: radius * (view[0][0].abs() + view[1][1].abs()) * 0.5,
            stops: stops.clone(),
        },
    }
}
