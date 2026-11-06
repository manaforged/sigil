use crate::kernel::{NodeBuilder, container};
use crate::layout::{Dimension, JustifyContent, LayoutStyle};
use crate::style::{Token, space};
use crate::widgets::Widget;

use super::axis::{legend, placed_axis, y_axis};
use super::bars::{self, GROUP_GAP};
use super::decimate::Candle;
use super::interact::Interact;
use super::marks::Placed;
use super::scatter::{self, Spans};
use super::span::ValueSpan;
use super::tips::Tips;
use super::{ChartMark, ChartView, Stack, marks, pie, readout};

pub(super) const AXIS: f32 = 32.0;
const PLOT_PAD: f32 = space(2);
pub(super) const X_AXIS: f32 = space(8);
const PAD: f32 = space(3);
const COMPACT_PAD: f32 = space(1);
const COMPACT_HEIGHT: f32 = 120.0;
const MIN_PLOT: f32 = space(8);
const MID_TICK_MIN: f32 = space(12);
const LEGEND: f32 = space(6);
const MIN_LABEL_PX: f32 = 32.0;
const FLUID_MAX_LABELS: usize = 12;
const FLUID_COLUMNS: usize = 1024;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct PlotFrame {
    pub(super) width: Option<f32>,
    pub(super) height: f32,
    pub(super) whole: bool,
}

impl PlotFrame {
    pub(super) fn padding(self) -> f32 {
        if self.height < COMPACT_HEIGHT {
            COMPACT_PAD
        } else {
            PAD
        }
    }

    fn mid_tick(self, span: ValueSpan, legend: bool) -> bool {
        let midpoint = span.lo.midpoint(span.hi);
        self.plot_height(legend) >= MID_TICK_MIN && (!self.whole || midpoint.fract() == 0.0)
    }

    fn inner(self, legend: bool) -> f32 {
        let legend = if legend { LEGEND } else { 0.0 };
        2.0f32.mul_add(-self.padding(), self.height) - legend
    }

    fn shows_x_axis(self, legend: bool) -> bool {
        self.inner(legend) >= X_AXIS + MIN_PLOT
    }

    fn plot_height(self, legend: bool) -> f32 {
        let axis = if self.shows_x_axis(legend) {
            X_AXIS
        } else {
            0.0
        };
        self.inner(legend) - axis
    }

    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "the label budget is a small non-negative count"
    )]
    fn label_stride(self, count: usize) -> usize {
        let budget = self.width.map_or(FLUID_MAX_LABELS, |width| {
            let span = 2.0f32.mul_add(-(self.padding() + PLOT_PAD), width) - AXIS - space(2);
            (span / MIN_LABEL_PX).floor().max(1.0) as usize
        });
        count.div_ceil(budget.max(1)).max(1)
    }

    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a widget width is a small non-negative pixel count"
    )]
    fn columns(self) -> usize {
        self.width
            .map_or(FLUID_COLUMNS, |width| width.max(1.0) as usize)
    }
}

pub(super) struct Plot<'a> {
    pub(super) series: &'a [ChartView<'a>],
    pub(super) labels: &'a [String],
    pub(super) positions: Option<&'a [f32]>,
    pub(super) mark: ChartMark,
    pub(super) stack: Stack,
    pub(super) count: usize,
    pub(super) total: usize,
    pub(super) frame: PlotFrame,
}

pub(super) fn draw(plot: &Plot<'_>, interact: &Interact) -> Vec<NodeBuilder> {
    let donut = match plot.mark {
        ChartMark::Pie => false,
        ChartMark::Donut => true,
        _ => return cartesian(plot, interact),
    };
    let slices = pie::Slices {
        values: plot.series.first().map_or(&[], |item| item.values),
        labels: plot.labels,
        side: plot.frame.inner(false).max(MIN_PLOT),
        donut,
    };
    pie::draw(&slices, interact)
}

fn placed<'a>(plot: &Plot<'a>) -> Option<Placed<'a>> {
    let at = plot
        .positions
        .filter(|_| matches!(plot.mark, ChartMark::Line | ChartMark::Area))?;
    Some(Placed {
        at,
        span: ValueSpan::scale(at.iter()),
    })
}

fn tips(plot: &Plot<'_>, spans: Spans, placed: Option<Placed<'_>>) -> Tips {
    let (series, labels) = (plot.series, plot.labels);
    match (plot.mark, placed) {
        (ChartMark::Candle, _) => readout::candles(series, labels),
        (ChartMark::Scatter, _) => readout::points(series, spans),
        (_, Some(placed)) => readout::placed(
            series,
            labels,
            (0..placed.at.len())
                .map(|index| placed.fraction(index))
                .collect(),
        ),
        _ => readout::columns(series, labels, plot.count),
    }
}

fn cartesian(plot: &Plot<'_>, interact: &Interact) -> Vec<NodeBuilder> {
    let (series, frame, count) = (plot.series, plot.frame, plot.count);
    let span = ValueSpan::of(series, plot.mark, plot.stack);
    let spans = Spans::of(series, span);
    let columns = frame.columns();
    let placed = placed(plot);
    let area = match plot.mark {
        ChartMark::Bar => bars::bars(series, count, span, plot.stack, PLOT_PAD),
        ChartMark::Candle => candles(series, span, columns),
        ChartMark::Scatter => dots(series, spans),
        _ => traces(plot, span, columns, placed),
    };
    let area = interact.attach(area, plot.total, PLOT_PAD, || tips(plot, spans, placed));
    let has_legend = series.iter().any(|item| !item.legend.is_empty());
    let x_axis_on = frame.shows_x_axis(has_legend);
    let mut column = container()
        .layout(
            LayoutStyle::flex_column()
                .grow(1.0)
                .min_size(0.0, 0.0)
                .fill_height(),
        )
        .child(area);
    if x_axis_on {
        column = column.child(bottom_axis(plot, spans, placed));
    }
    let mid_tick = frame.mid_tick(span, has_legend);
    let mut nodes = vec![
        container()
            .layout(
                LayoutStyle::flex_row()
                    .fill_width()
                    .grow(1.0)
                    .min_size(0.0, 0.0)
                    .gap(space(2)),
            )
            .child(y_axis(span, x_axis_on, mid_tick))
            .child(column),
    ];
    if has_legend {
        nodes.push(legend(series));
    }
    nodes
}

fn bottom_axis(plot: &Plot<'_>, spans: Spans, placed: Option<Placed<'_>>) -> NodeBuilder {
    let (count, stride) = (plot.count, plot.frame.label_stride(plot.count));
    match (plot.mark, placed) {
        (ChartMark::Scatter, _) => scatter::x_axis(spans.x, X_AXIS, PLOT_PAD),
        (ChartMark::Bar, _) => x_axis(plot.labels, count, GROUP_GAP, stride),
        (_, Some(placed)) => placed_axis(plot.labels, placed, stride, PLOT_PAD),
        _ => x_axis(plot.labels, count, 0.0, stride),
    }
}

fn x_axis(labels: &[String], count: usize, gap: f32, stride: usize) -> NodeBuilder {
    let mut row = container().layout(
        LayoutStyle::flex_row()
            .fill_width()
            .height(X_AXIS)
            .shrink(0.0)
            .padding_x(PLOT_PAD)
            .gap(gap),
    );
    for slot in 0..count.div_ceil(stride) {
        let first = slot * stride;
        let width = (count - first).min(stride);
        let text = labels
            .get(first)
            .cloned()
            .unwrap_or_else(|| (first + 1).to_string());
        row = row.child(axis_slot(text, width));
    }
    row
}

#[allow(
    clippy::cast_precision_loss,
    reason = "a label slot spans a small point count"
)]
fn axis_slot(text: String, width: usize) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_row()
                .grow(width as f32)
                .flex_basis(Dimension::Px(0.0))
                .min_size(0.0, 0.0)
                .justify_content(JustifyContent::Center),
        )
        .child(
            crate::kernel::text(text)
                .wrap()
                .class("text-xs")
                .class(Token::Gray11.text()),
        )
}

fn traces(
    plot: &Plot<'_>,
    span: ValueSpan,
    columns: usize,
    placed: Option<Placed<'_>>,
) -> NodeBuilder {
    let filled = plot.mark == ChartMark::Area;
    let stacked = filled && plot.stack == Stack::Stacked;
    let raw = plot
        .series
        .iter()
        .map(|item| item.values)
        .collect::<Vec<_>>();
    let tops = stacked.then(|| marks::stack_tops(&raw));
    let values = tops
        .as_ref()
        .map_or(raw, |tops| tops.iter().map(Vec::as_slice).collect());
    let shown = plot
        .series
        .iter()
        .zip(values)
        .filter(|(_, values)| !values.is_empty())
        .collect::<Vec<_>>();
    let trace = |values| marks::Trace {
        values,
        count: plot.count,
        span,
        columns,
        placed,
    };
    let mut layer = container().layout(LayoutStyle::block().fill());
    if filled {
        let mut under = shown.clone();
        if stacked {
            under.reverse();
        }
        for (item, values) in under {
            layer = layer.child(marks::area(&trace(values), item.hue.soft()));
        }
    }
    for (item, values) in shown {
        layer = layer.child(marks::line(&trace(values), item.hue.token()));
    }
    plot_area(layer)
}

fn dots(series: &[ChartView<'_>], spans: Spans) -> NodeBuilder {
    let layer = container().layout(LayoutStyle::block().fill());
    plot_area(
        series
            .iter()
            .map(|item| scatter::dots(item, spans))
            .fold(layer, NodeBuilder::child),
    )
}

fn candles(series: &[ChartView<'_>], span: ValueSpan, columns: usize) -> NodeBuilder {
    let rows = series
        .first()
        .map(|item| Candle::rows(item.values))
        .unwrap_or_default();
    let layer = container().layout(LayoutStyle::block().fill());
    plot_area(
        marks::candles(&rows, span, columns)
            .into_iter()
            .fold(layer, NodeBuilder::child),
    )
}

fn plot_area(layer: NodeBuilder) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::block()
                .fill_width()
                .grow(1.0)
                .min_size(0.0, 0.0)
                .padding_x(PLOT_PAD),
        )
        .class("border-b")
        .class(Token::Gray6.border())
        .child(layer)
}
