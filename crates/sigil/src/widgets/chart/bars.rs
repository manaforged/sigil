use crate::kernel::{NodeBuilder, container};
use crate::layout::{AlignItems, Dimension, LayoutStyle};
use crate::style::{Radius, Token};
use crate::widgets::Widget;

use super::axis::format_tick;
use super::palette::Hue;
use super::span::ValueSpan;
use super::{ChartView, Stack};

pub(super) const GROUP_GAP: f32 = crate::style::space(2);
const SERIES_GAP: f32 = 2.0;
const PERCENT: f32 = 100.0;
const MIN_GROW: f32 = 0.01;
const MIN_SEGMENT: f32 = 2.0;

pub(super) fn bars(
    series: &[ChartView<'_>],
    count: usize,
    span: ValueSpan,
    stack: Stack,
    pad: f32,
) -> NodeBuilder {
    let mut row = container()
        .layout(
            LayoutStyle::flex_row()
                .fill_width()
                .grow(1.0)
                .min_size(0.0, 0.0)
                .gap(GROUP_GAP)
                .align_items(AlignItems::End)
                .padding(pad),
        )
        .class("border-b")
        .class(Token::Gray6.border());
    for index in 0..count {
        let mut group = container().layout(
            LayoutStyle::flex_row()
                .grow(1.0)
                .flex_basis(Dimension::Px(0.0))
                .min_size(0.0, 0.0)
                .fill_height()
                .gap(SERIES_GAP)
                .align_items(AlignItems::End),
        );
        let cells = series.iter().map(|item| Cell {
            hue: item.hue,
            name: item.name,
            value: item.values.get(index).copied().unwrap_or(0.0),
        });
        match stack {
            Stack::Grouped => {
                for cell in cells {
                    group = group.child(column(span, &[cell]));
                }
            }
            Stack::Stacked => group = group.child(column(span, &cells.collect::<Vec<_>>())),
        }
        row = row.child(group);
    }
    row
}

#[derive(Clone, Copy)]
struct Cell<'a> {
    hue: Hue,
    name: &'a str,
    value: f32,
}

impl Cell<'_> {
    fn shown(&self) -> bool {
        self.value != 0.0 && self.value.is_finite()
    }

    fn segment(&self, grow: f32) -> NodeBuilder {
        let caption = if self.name.is_empty() {
            format_tick(self.value)
        } else {
            format!("{} {}", self.name, format_tick(self.value))
        };
        container()
            .label(caption)
            .layout(
                LayoutStyle::block()
                    .fill_width()
                    .grow(grow)
                    .min_size(0.0, MIN_SEGMENT),
            )
            .class([Radius::Sm.atom(), self.hue.token().bg()])
            .tone_scope(self.hue.tone)
    }
}

fn column(span: ValueSpan, cells: &[Cell<'_>]) -> NodeBuilder {
    let column = container().layout(
        LayoutStyle::flex_column()
            .grow(1.0)
            .flex_basis(Dimension::Px(0.0))
            .min_size(0.0, 0.0)
            .fill_height(),
    );
    let shown = cells.iter().filter(|cell| cell.shown()).collect::<Vec<_>>();
    if shown.is_empty() {
        return column;
    }
    let unit = PERCENT / span.width();
    let up: f32 = shown.iter().map(|cell| cell.value.max(0.0)).sum();
    let down: f32 = shown.iter().map(|cell| cell.value.min(0.0)).sum();
    let above = (span.hi - up.clamp(0.0, span.hi)) * unit;
    let below = (down.clamp(span.lo, 0.0) - span.lo) * unit;
    let rising = shown.iter().rev().filter(|cell| cell.value > 0.0);
    let falling = shown.iter().filter(|cell| cell.value < 0.0);
    rising
        .chain(falling)
        .fold(column.child(spacer(above)), |column, cell| {
            column.child(cell.segment((cell.value.abs().min(span.width()) * unit).max(1.0)))
        })
        .child(spacer(below))
}

fn spacer(grow: f32) -> NodeBuilder {
    container().layout(LayoutStyle::block().fill_width().grow(grow.max(MIN_GROW)))
}
