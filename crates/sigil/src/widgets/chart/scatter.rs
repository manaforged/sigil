use crate::layout::Space;
use std::rc::Rc;

use crate::kernel::{NodeBuilder, container};
use crate::layout::{Dimension, LayoutStyle, Position};
use crate::style::{Radius, StyleAtom};
use crate::widgets::Widget;

use super::super::vector::path;
use super::ChartView;
use super::marks::{INSET, VIEW_H, VIEW_H_F, VIEW_W, VIEW_W_F, layer_layout, y_of};
use super::scales::{Dot, Fit};
use super::span::ValueSpan;

const DOT: f32 = 7.0;
const DOT_RUN: f32 = 0.01;
const MARKER: f32 = 7.0;
const RING: i16 = 2;

#[derive(Clone, Copy)]
pub(super) struct Spans {
    pub(super) x: ValueSpan,
    pub(super) y: ValueSpan,
}

impl Spans {
    pub(super) fn of(series: &[ChartView<'_>], y: ValueSpan, x: Fit) -> Self {
        Self {
            x: ValueSpan::scatter_x(series, x),
            y,
        }
    }

    fn x_of(self, value: f32) -> f32 {
        self.x
            .share(value)
            .mul_add(2.0f32.mul_add(-INSET, VIEW_W_F), INSET)
    }

    pub(super) fn view(self, x: f32, y: f32) -> (f32, f32) {
        (self.x_of(x), y_of(self.y, y))
    }
}

pub(super) fn pairs(values: &[f32]) -> impl Iterator<Item = (f32, f32)> + '_ {
    values
        .as_chunks::<2>()
        .0
        .iter()
        .map(|&pair| pair.into())
        .filter(|(x, y)| x.is_finite() && y.is_finite())
}

pub(super) fn dots(
    layer: NodeBuilder,
    item: &ChartView<'_>,
    spans: Spans,
    dot: Dot,
) -> NodeBuilder {
    if dot != Dot::Disc {
        return markers(layer, item, spans, dot);
    }
    let shape = pairs(item.values).fold(path(VIEW_W, VIEW_H), |shape, (x, y)| {
        let (x, y) = spans.view(x, y);
        shape.move_to(x, y).line_to(x + DOT_RUN, y)
    });
    layer.child(
        shape
            .stroke(item.hue.paint(), DOT)
            .non_scaling_stroke()
            .layout(layer_layout()),
    )
}

fn markers(layer: NodeBuilder, item: &ChartView<'_>, spans: Spans, dot: Dot) -> NodeBuilder {
    let paint = match dot {
        Dot::Ring => [
            Radius::Full.atom(),
            StyleAtom::BorderWidth(RING),
            StyleAtom::BorderColor(item.hue.color()),
        ],
        Dot::Square | Dot::Disc => [
            Radius::None.atom(),
            item.hue.bg(),
            StyleAtom::BorderWidth(0),
        ],
    };
    pairs(item.values).fold(layer, |layer, (x, y)| {
        let (x, y) = spans.view(x, y);
        layer.child(
            container()
                .layout(
                    LayoutStyle::block()
                        .position(Position::Absolute)
                        .inset_left(Dimension::Percent(x / VIEW_W_F))
                        .inset_top(Dimension::Percent(y / VIEW_H_F))
                        .margin_left(Space::px(-MARKER / 2.0))
                        .margin_top(Space::px(-MARKER / 2.0))
                        .fixed_size(MARKER, MARKER),
                )
                .class(paint),
        )
    })
}

pub(super) fn normalized(series: &[ChartView<'_>], spans: Spans) -> Rc<[(f32, f32)]> {
    series
        .iter()
        .flat_map(|item| pairs(item.values))
        .map(|(x, y)| {
            let (x, y) = spans.view(x, y);
            (x / VIEW_W_F, y / VIEW_H_F)
        })
        .collect()
}
