use crate::kernel::{NodeBuilder, container};
use crate::layout::Space;
use crate::layout::{AlignItems, Dimension, JustifyContent, LayoutStyle};
use crate::style::{Radius, Token};
use crate::widgets::Widget;

use super::super::vector::{PathNode, path};
use super::axis::format_tick;
use super::marks::{VIEW_H, VIEW_H_F, VIEW_W, VIEW_W_F, layer_layout};
use super::palette::Hue;
use super::scales::Orient;
use super::span::ValueSpan;
use super::{ChartView, Stack};

const BAND: f32 = 0.9;
const PERCENT: f32 = 100.0;
const MIN_GROW: f32 = 0.0001;
const MIN_BAR: f32 = 1.0;
const RULE: f32 = 1.0;

pub(super) struct Bars<'a> {
    pub(super) series: &'a [ChartView<'a>],
    pub(super) count: usize,
    pub(super) span: ValueSpan,
    pub(super) stack: Stack,
    pub(super) pad: f32,
    pub(super) orient: Orient,
}

impl Bars<'_> {
    const fn vertical(&self) -> bool {
        matches!(self.orient, Orient::Vertical)
    }

    fn track(&self) -> LayoutStyle {
        let track = if self.vertical() {
            LayoutStyle::flex_row()
        } else {
            LayoutStyle::flex_column()
        };
        track.fill()
    }

    fn slot(&self) -> LayoutStyle {
        let slot = if self.vertical() {
            LayoutStyle::flex_row().fill_height()
        } else {
            LayoutStyle::flex_column().fill_width()
        };
        slot.grow(1.0)
            .flex_basis(Dimension::Px(0.0))
            .min_size(0.0, 0.0)
            .justify_content(JustifyContent::Center)
            .align_items(AlignItems::Stretch)
    }

    fn group(&self) -> LayoutStyle {
        if self.vertical() {
            LayoutStyle::flex_row()
                .width(Dimension::Percent(BAND))
                .min_width(MIN_BAR)
                .fill_height()
                .shrink(0.0)
        } else {
            LayoutStyle::flex_column()
                .height(Dimension::Percent(BAND))
                .min_height(MIN_BAR)
                .fill_width()
                .shrink(0.0)
        }
    }

    fn lane(&self) -> LayoutStyle {
        let lane = if self.vertical() {
            LayoutStyle::flex_column().fill_height()
        } else {
            LayoutStyle::flex_row().fill_width()
        };
        lane.grow(1.0)
            .flex_basis(Dimension::Px(0.0))
            .min_size(0.0, 0.0)
    }
}

pub(super) fn bars(plot: &Bars<'_>) -> NodeBuilder {
    let mut track = container().layout(plot.track());
    for index in 0..plot.count {
        let cells = plot
            .series
            .iter()
            .map(|item| Cell {
                hue: item.hue,
                name: item.name,
                value: item.values.get(index).copied().unwrap_or(0.0),
            })
            .collect::<Vec<_>>();
        let mut group = container().layout(plot.group());
        match plot.stack {
            Stack::Grouped => {
                for cell in cells {
                    group = group.child(lane(plot, &[cell]));
                }
            }
            Stack::Stacked => group = group.child(lane(plot, &cells)),
        }
        track = track.child(container().layout(plot.slot()).child(group));
    }
    let mut area = container()
        .layout(
            LayoutStyle::block()
                .fill_width()
                .grow(1.0)
                .min_size(0.0, 0.0)
                .padding(Space::px(plot.pad)),
        )
        .class("border-b")
        .class(Token::Gray6.border())
        .child(track);
    if plot.span.lo < 0.0 && plot.span.hi > 0.0 {
        area = area.child(zero_rule(plot));
    }
    area
}

fn zero_rule(plot: &Bars<'_>) -> PathNode {
    let at = plot.span.fraction(0.0);
    let line = if plot.vertical() {
        let y = (1.0 - at) * VIEW_H_F;
        path(VIEW_W, VIEW_H).move_to(0.0, y).line_to(VIEW_W_F, y)
    } else {
        let x = at * VIEW_W_F;
        path(VIEW_W, VIEW_H).move_to(x, 0.0).line_to(x, VIEW_H_F)
    };
    line.stroke(Token::Gray11, RULE)
        .non_scaling_stroke()
        .layout(layer_layout())
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

    fn segment(&self, grow: f32, vertical: bool) -> NodeBuilder {
        let caption = if self.name.is_empty() {
            format_tick(self.value)
        } else {
            format!("{} {}", self.name, format_tick(self.value))
        };
        let layout = if vertical {
            LayoutStyle::block().fill_width().min_size(0.0, MIN_BAR)
        } else {
            LayoutStyle::block().fill_height().min_size(MIN_BAR, 0.0)
        };
        container()
            .label(caption)
            .layout(layout.grow(grow).flex_basis(Dimension::Px(0.0)))
            .class([Radius::Sm.atom(), self.hue.bg()])
            .tone_scope(self.hue.tone)
    }
}

type Piece<'a> = (f32, f32, Cell<'a>);

fn pieces<'a>(cells: &[Cell<'a>], vertical: bool, span: ValueSpan) -> Vec<Piece<'a>> {
    let mut outward = cells
        .iter()
        .filter(|cell| cell.shown())
        .copied()
        .collect::<Vec<_>>();
    if vertical {
        outward.reverse();
    }
    let clip = |from: f32, to: f32| (from.max(span.lo), to.min(span.hi));
    let (mut up, mut down) = (0.0_f32, 0.0_f32);
    let (mut negative, mut positive) = (Vec::new(), Vec::new());
    for cell in outward {
        if cell.value > 0.0 {
            let (from, to) = clip(up, up + cell.value);
            positive.push((from, to, cell));
            up += cell.value;
        } else {
            let (from, to) = clip(down + cell.value, down);
            negative.push((from, to, cell));
            down += cell.value;
        }
    }
    negative.reverse();
    negative
        .into_iter()
        .chain(positive)
        .filter(|(from, to, _)| to > from)
        .collect()
}

fn lane(plot: &Bars<'_>, cells: &[Cell<'_>]) -> NodeBuilder {
    let lane = container().layout(plot.lane());
    let span = plot.span;
    let parts = pieces(cells, plot.vertical(), span);
    let (Some(first), Some(last)) = (parts.first(), parts.last()) else {
        return lane;
    };
    let unit = PERCENT / span.width();
    let mut nodes = vec![spacer((first.0 - span.lo) * unit)];
    nodes.extend(
        parts
            .iter()
            .map(|(from, to, cell)| cell.segment((to - from) * unit, plot.vertical())),
    );
    nodes.push(spacer((span.hi - last.1) * unit));
    if plot.vertical() {
        nodes.reverse();
    }
    nodes.into_iter().fold(lane, NodeBuilder::child)
}

fn spacer(grow: f32) -> NodeBuilder {
    container().layout(
        LayoutStyle::block()
            .grow(grow.max(MIN_GROW))
            .flex_basis(Dimension::Px(0.0))
            .min_size(0.0, 0.0),
    )
}
