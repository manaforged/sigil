use std::sync::Arc;

use smallvec::SmallVec;

use super::key::{hash_cmd, mix, mix_f32, mix_fill, mix_stroke};
use super::{PathBuilder, PathCmd, PathFill};
use crate::fnv::OFFSET;
use crate::geometry::Point;
use crate::scene::command::{Gradient, GradientKind, GradientStop, PathPaint, Transform};
use crate::style::{Color, ColorAtom, Theme};

type Resolved = (Option<PathPaint>, Option<(Color, f32)>);

pub(super) fn resolve(
    fill: Option<&PathFill>,
    stroke: Option<(ColorAtom, f32)>,
    theme: &Theme,
) -> Resolved {
    let point = |(x, y): (f32, f32)| Point::new(x, y);
    let stops = |stops: &[(f32, Color)]| -> SmallVec<[GradientStop; 4]> {
        stops
            .iter()
            .map(|&(offset, color)| GradientStop::new(offset, color))
            .collect()
    };
    let fill = fill.map(|fill| match fill {
        PathFill::Solid(color) => PathPaint::Solid(theme.color(*color)),
        PathFill::Linear {
            start,
            end,
            stops: list,
        } => {
            let kind = GradientKind::Linear {
                start: point(*start),
                end: point(*end),
            };
            PathPaint::Gradient(Gradient::new(kind, stops(list)))
        }
        PathFill::Radial {
            center,
            radius,
            stops: list,
        } => {
            let kind = GradientKind::Radial {
                center: point(*center),
                radius: [*radius; 2],
            };
            PathPaint::Gradient(Gradient::new(kind, stops(list)))
        }
    });
    let stroke = stroke.map(|(color, width)| (theme.color(color), width));
    (fill, stroke)
}

pub(super) fn path_key(
    node: &PathBuilder,
    fill: Option<&PathPaint>,
    stroke: Option<(Color, f32)>,
) -> u64 {
    let mut hash = crate::fnv::bytes(OFFSET, &node.width.to_le_bytes());
    hash = crate::fnv::bytes(hash, &node.height.to_le_bytes());
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

pub(super) fn theme_key(node: &PathBuilder, theme: &Theme) -> u64 {
    let (fill, stroke) = resolve(node.fill.as_ref(), node.stroke, theme);
    let mut hash = OFFSET;
    mix_stroke(&mut hash, stroke);
    mix_fill(&mut hash, fill.as_ref());
    hash
}

const fn apply([[sx, kx, tx], [ky, sy, ty]]: Transform, (x, y): (f32, f32)) -> (f32, f32) {
    (
        sx.mul_add(x, kx.mul_add(y, tx)),
        ky.mul_add(x, sy.mul_add(y, ty)),
    )
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
        PathPaint::Gradient(gradient) => {
            let mut mapped = gradient.clone();
            mapped.kind = match gradient.kind {
                GradientKind::Linear { start, end } => GradientKind::Linear {
                    start: at(start),
                    end: at(end),
                },
                GradientKind::Radial { center, radius } => GradientKind::Radial {
                    center: at(center),
                    radius: [radius[0] * view[0][0].abs(), radius[1] * view[1][1].abs()],
                },
                GradientKind::Conic { center, angle } => GradientKind::Conic {
                    center: at(center),
                    angle,
                },
            };
            PathPaint::Gradient(mapped)
        }
    }
}
