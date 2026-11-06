use std::f32::consts::TAU;
use std::rc::Rc;

use crate::kernel::{NodeBuilder, Role, text};
use crate::layout::{LayoutStyle, Position};
use crate::style::{Token, space};
use crate::widgets::Widget;

use super::super::popup::surface;

const OFFSET: f32 = space(2);
const NEAR: f32 = 0.08;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Probe {
    pub(super) x: f32,
    pub(super) y: f32,
    pub(super) width: f32,
    pub(super) height: f32,
}

#[derive(Clone)]
pub(super) enum Pick {
    Columns(usize),
    Slices { ends: Rc<[f32]>, hole: f32 },
    Points(Rc<[(f32, f32)]>),
}

pub(super) type Lines = Rc<dyn Fn(usize) -> Vec<String>>;

#[derive(Clone)]
pub(super) struct Tips {
    pub(super) pick: Pick,
    pub(super) lines: Lines,
}

impl Tips {
    pub(super) fn index(&self, at: Probe, pad: f32) -> Option<usize> {
        match &self.pick {
            Pick::Columns(count) => column(at, *count, pad),
            Pick::Slices { ends, hole } => slice(at, ends, *hole),
            Pick::Points(points) => nearest(at, points, pad),
        }
    }
}

fn span_px(width: f32, pad: f32) -> f32 {
    2.0f32.mul_add(-pad, width).max(1.0)
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "the probe index is clamped to the visible point count"
)]
fn column(at: Probe, count: usize, pad: f32) -> Option<usize> {
    if count == 0 {
        return None;
    }
    let fraction = ((at.x - pad) / span_px(at.width, pad)).clamp(0.0, 1.0);
    let index = fraction
        .mul_add(count as f32, -0.5)
        .round()
        .clamp(0.0, (count - 1) as f32);
    Some(index as usize)
}

fn slice(at: Probe, ends: &[f32], hole: f32) -> Option<usize> {
    let radius = at.width.min(at.height) / 2.0;
    let (dx, dy) = (at.x - at.width / 2.0, at.y - at.height / 2.0);
    let distance = dx.hypot(dy);
    if radius <= 0.0 || distance > radius || distance < radius * hole {
        return None;
    }
    let turn = dx.atan2(-dy).rem_euclid(TAU) / TAU;
    ends.iter().position(|end| turn <= *end)
}

fn nearest(at: Probe, points: &[(f32, f32)], pad: f32) -> Option<usize> {
    let fx = (at.x - pad) / span_px(at.width, pad);
    let fy = at.y / at.height.max(1.0);
    let gap = |&(x, y): &(f32, f32)| (x - fx).hypot(y - fy);
    let (index, point) = points
        .iter()
        .enumerate()
        .min_by(|a, b| gap(a.1).total_cmp(&gap(b.1)))?;
    (gap(point) <= NEAR).then_some(index)
}

pub(super) fn card(at: Probe, lines: Vec<String>) -> Option<NodeBuilder> {
    if lines.is_empty() {
        return None;
    }
    let mut layout = LayoutStyle::flex_column()
        .position(Position::Absolute)
        .padding_x(space(2))
        .padding_y(space(1));
    layout = if at.x > at.width / 2.0 {
        layout.inset_right(at.width - at.x + OFFSET)
    } else {
        layout.inset_left(at.x + OFFSET)
    };
    layout = if at.y > at.height / 2.0 {
        layout.inset_bottom(at.height - at.y + OFFSET)
    } else {
        layout.inset_top(at.y + OFFSET)
    };
    let body = lines
        .into_iter()
        .fold(surface(Role::Tooltip), |body, line| {
            body.child(
                text(line)
                    .nowrap()
                    .class("text-xs")
                    .class(Token::OnSurface.text()),
            )
        });
    Some(body.layout(layout))
}
