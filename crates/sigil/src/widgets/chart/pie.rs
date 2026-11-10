use crate::layout::Space;
use std::f32::consts::TAU;
use std::rc::Rc;

use crate::kernel::{NodeBuilder, container, text};
use crate::layout::{AlignItems, JustifyContent, LayoutStyle};
use crate::style::{Radius, Token};
use crate::widgets::Widget;

use super::super::vector::{PathNode, path};
use super::axis::format_tick;
use super::interact::Interact;
use super::marks::layer_layout;
use super::palette::Hue;
use super::summary::empty;
use super::tips::{Pick, Tips};

const VIEW: u32 = 200;
const CENTER: f32 = 100.0;
const RADIUS: f32 = 96.0;
const HOLE: f32 = 0.6;
const STEPS_PER_TURN: f32 = 96.0;
const EDGE: f32 = 1.5;
const SWATCH: f32 = 8.0;
const PERCENT: f32 = 100.0;

pub(super) struct Slices<'a> {
    pub(super) values: &'a [f32],
    pub(super) labels: &'a [String],
    pub(super) side: f32,
    pub(super) donut: bool,
    pub(super) hues: Option<&'a [Hue]>,
}

impl Slices<'_> {
    #[allow(
        clippy::cast_possible_truncation,
        reason = "a share is a fraction in 0..=1"
    )]
    fn shares(&self) -> Option<Vec<f32>> {
        let total = total(self.values);
        (total > 0.0).then(|| {
            self.values
                .iter()
                .map(|value| (weight(*value) / total) as f32)
                .collect()
        })
    }

    fn hue(&self, index: usize) -> Hue {
        self.hues
            .and_then(|hues| hues.get(index).copied())
            .unwrap_or_else(|| Hue::nth(index))
    }

    fn label(&self, index: usize) -> String {
        self.labels
            .get(index)
            .cloned()
            .unwrap_or_else(|| (index + 1).to_string())
    }
}

fn weight(value: f32) -> f64 {
    if value.is_finite() {
        f64::from(value.max(0.0))
    } else {
        0.0
    }
}

fn total(values: &[f32]) -> f64 {
    values.iter().map(|value| weight(*value)).sum()
}

pub(super) fn draw(slices: &Slices<'_>, interact: &Interact) -> Vec<NodeBuilder> {
    let Some(shares) = slices.shares() else {
        return vec![empty()];
    };
    let hole = if slices.donut { HOLE } else { 0.0 };
    let ends = shares
        .iter()
        .scan(0.0_f32, |sum, share| {
            *sum += share;
            Some(*sum)
        })
        .collect::<Rc<[f32]>>();
    let mut picture = container().layout(
        LayoutStyle::block()
            .fixed_size(slices.side, slices.side)
            .shrink(0.0),
    );
    let mut start = 0.0;
    for (index, end) in ends.iter().enumerate() {
        picture = picture.child(wedge(start, *end, hole, slices.hue(index)));
        start = *end;
    }
    let lines = tip_lines(slices);
    let picture = interact.attach(picture, shares.len(), 0.0, move || Tips {
        pick: Pick::Slices { ends, hole },
        lines,
    });
    vec![
        container()
            .layout(
                LayoutStyle::flex_row()
                    .fill_width()
                    .grow(1.0)
                    .min_size(0.0, 0.0)
                    .gap(Space::Lg)
                    .align_items(AlignItems::Center),
            )
            .child(picture)
            .child(legend(slices, &shares)),
    ]
}

fn tip_lines(slices: &Slices<'_>) -> super::tips::Lines {
    let names = (0..slices.values.len())
        .map(|index| slices.label(index))
        .collect::<Vec<_>>();
    let values = slices.values.to_vec();
    let total = total(&values);
    Rc::new(move |index| {
        let (Some(name), Some(value)) = (names.get(index), values.get(index)) else {
            return Vec::new();
        };
        let percent = f64::from(*value) / total * f64::from(PERCENT);
        vec![
            name.clone(),
            format!("{} ({percent:.1}%)", format_tick(*value)),
        ]
    })
}

fn point(turn: f32, radius: f32) -> (f32, f32) {
    let (sin, cos) = (turn * TAU).sin_cos();
    (radius.mul_add(sin, CENTER), radius.mul_add(-cos, CENTER))
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "an arc takes a small positive step count"
)]
fn arc(shape: PathNode, from: f32, to: f32, radius: f32) -> PathNode {
    let steps = ((to - from).abs() * STEPS_PER_TURN).ceil().max(2.0) as usize;
    (0..=steps).fold(shape, |shape, step| {
        let (x, y) = point(from + (to - from) * step as f32 / steps as f32, radius);
        shape.line_to(x, y)
    })
}

fn wedge(start: f32, end: f32, hole: f32, hue: Hue) -> PathNode {
    let (x, y) = point(start, RADIUS * hole);
    let mut shape = arc(path(VIEW, VIEW).move_to(x, y), start, end, RADIUS);
    shape = if hole > 0.0 {
        arc(shape, end, start, RADIUS * hole)
    } else {
        shape.line_to(CENTER, CENTER)
    };
    shape
        .close()
        .fill(hue.paint())
        .stroke(Token::Surface, EDGE)
        .non_scaling_stroke()
        .layout(layer_layout())
}

fn legend(slices: &Slices<'_>, shares: &[f32]) -> NodeBuilder {
    let column = container().layout(
        LayoutStyle::flex_column()
            .grow(1.0)
            .min_size(0.0, 0.0)
            .gap(Space::Xs)
            .justify_content(JustifyContent::Center),
    );
    shares
        .iter()
        .enumerate()
        .fold(column, |column, (index, share)| {
            let value = slices.values.get(index).copied().unwrap_or(0.0);
            column.child(legend_row(
                &slices.label(index),
                &format!("{} · {:.0}%", format_tick(value), share * PERCENT),
                slices.hue(index),
            ))
        })
}

fn legend_row(name: &str, value: &str, hue: Hue) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_row()
                .gap(Space::px(6.0))
                .align_items(AlignItems::Center),
        )
        .child(
            container()
                .layout(LayoutStyle::block().fixed_size(SWATCH, SWATCH).shrink(0.0))
                .class([Radius::Sm.atom(), hue.bg()]),
        )
        .child(
            text(name.to_owned())
                .nowrap()
                .class("text-xs")
                .class(Token::Gray12.text()),
        )
        .child(
            text(value.to_owned())
                .nowrap()
                .class("text-xs")
                .class(Token::Gray11.text()),
        )
}
