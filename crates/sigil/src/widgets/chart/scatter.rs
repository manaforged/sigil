use std::rc::Rc;

use crate::kernel::{NodeBuilder, container, text};
use crate::layout::{JustifyContent, LayoutStyle};
use crate::style::Token;
use crate::widgets::Widget;

use super::super::vector::{PathNode, path};
use super::ChartView;
use super::axis::format_tick;
use super::marks::{INSET, VIEW_H, VIEW_H_F, VIEW_W, VIEW_W_F, layer_layout, y_of};
use super::span::ValueSpan;

const DOT: f32 = 7.0;
const DOT_RUN: f32 = 0.01;

#[derive(Clone, Copy)]
pub(super) struct Spans {
    pub(super) x: ValueSpan,
    pub(super) y: ValueSpan,
}

impl Spans {
    pub(super) fn of(series: &[ChartView<'_>], y: ValueSpan) -> Self {
        Self {
            x: ValueSpan::scatter_x(series),
            y,
        }
    }

    fn x_of(self, value: f32) -> f32 {
        self.x
            .fraction(value)
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
        .map(|&[x, y]| (x, y))
        .filter(|(x, y)| x.is_finite() && y.is_finite())
}

pub(super) fn dots(item: &ChartView<'_>, spans: Spans) -> PathNode {
    let shape = pairs(item.values).fold(path(VIEW_W, VIEW_H), |shape, (x, y)| {
        let (x, y) = spans.view(x, y);
        shape.move_to(x, y).line_to(x + DOT_RUN, y)
    });
    shape
        .stroke(item.hue.token(), DOT)
        .non_scaling_stroke()
        .layout(layer_layout())
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

pub(super) fn x_axis(span: ValueSpan, height: f32, pad: f32) -> NodeBuilder {
    let ticks = [span.lo, span.lo.midpoint(span.hi), span.hi];
    ticks.into_iter().fold(
        container().layout(
            LayoutStyle::flex_row()
                .fill_width()
                .height(height)
                .shrink(0.0)
                .padding_x(pad)
                .justify_content(JustifyContent::SpaceBetween),
        ),
        |row, value| {
            row.child(
                text(format_tick(value))
                    .nowrap()
                    .class("text-xs")
                    .class(Token::Gray11.text()),
            )
        },
    )
}
