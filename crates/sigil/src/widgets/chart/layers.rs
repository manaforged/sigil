use crate::kernel::{NodeBuilder, container};
use crate::layout::LayoutStyle;
use crate::layout::Space;
use crate::style::Token;
use crate::widgets::Widget;

use super::decimate::Candle;
use super::marks::{self, Placed};
use super::scales::Dot;
use super::scatter::{self, Spans};
use super::span::ValueSpan;
use super::{ChartMark, ChartView, Stack};

pub(super) struct Traces<'a> {
    pub(super) series: &'a [ChartView<'a>],
    pub(super) mark: ChartMark,
    pub(super) stack: Stack,
    pub(super) count: usize,
    pub(super) span: ValueSpan,
    pub(super) columns: usize,
    pub(super) placed: Option<Placed<'a>>,
}

pub(super) fn traces(plot: &Traces<'_>, pad: f32) -> NodeBuilder {
    let filled = plot.mark == ChartMark::Area;
    let stacked = filled && plot.stack == Stack::Stacked;
    let raw = plot
        .series
        .iter()
        .map(|item| item.values)
        .collect::<Vec<_>>();
    let tops = stacked.then(|| marks::stack_tops(&raw));
    let heights = tops
        .as_ref()
        .map_or(raw, |tops| tops.iter().map(Vec::as_slice).collect());
    let shown = plot
        .series
        .iter()
        .zip(heights)
        .filter(|(_, values)| !values.is_empty())
        .collect::<Vec<_>>();
    let trace = |values| marks::Trace {
        values,
        count: plot.count,
        span: plot.span,
        columns: plot.columns,
        placed: plot.placed,
    };
    let mut layer = container().layout(LayoutStyle::block().fill());
    if filled {
        for (item, values) in &shown {
            layer = layer.child(marks::area(&trace(values), item.hue.soft()));
        }
    }
    for (item, values) in shown {
        layer = layer.child(marks::line(&trace(values), item.hue.paint()));
    }
    plot_area(layer, pad)
}

pub(super) fn dots(series: &[ChartView<'_>], spans: Spans, dot: Dot, pad: f32) -> NodeBuilder {
    let layer = container().layout(LayoutStyle::block().fill());
    plot_area(
        series
            .iter()
            .fold(layer, |layer, item| scatter::dots(layer, item, spans, dot)),
        pad,
    )
}

pub(super) fn candles(
    series: &[ChartView<'_>],
    span: ValueSpan,
    columns: usize,
    pad: f32,
) -> NodeBuilder {
    let rows = series
        .first()
        .map(|item| Candle::rows(item.values))
        .unwrap_or_default();
    let layer = container().layout(LayoutStyle::block().fill());
    plot_area(
        marks::candles(&rows, span, columns)
            .into_iter()
            .fold(layer, NodeBuilder::child),
        pad,
    )
}

fn plot_area(layer: NodeBuilder, pad: f32) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::block()
                .fill_width()
                .grow(1.0)
                .min_size(0.0, 0.0)
                .padding(Space::px(pad)),
        )
        .class("border-b")
        .class(Token::Gray6.border())
        .child(layer)
}
