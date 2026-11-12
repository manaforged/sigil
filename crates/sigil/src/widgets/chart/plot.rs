use crate::kernel::{NodeBuilder, container};
use crate::layout::Space;
use crate::layout::{JustifyContent, LayoutStyle, Length};
use crate::style::{Token, space};
use crate::widgets::Layout;

use super::axis::{Gutter, axis_width, category_axis, crowded, legend, title, x_axis, y_axis};
use super::bars::{self, Bars};
use super::interact::Interact;
use super::layers::{self, Traces};
use super::marks::Placed;
use super::scales::{Orient, Scales};
use super::scatter::Spans;
use super::span::ValueSpan;
use super::ticks::{self, MIN_TICKS, Tick};
use super::tips::Tips;
use super::{ChartMark, ChartView, Stack, clock, pie, readout};

pub(super) const AXIS: f32 = 32.0;
const PLOT_PAD: f32 = space(2);
pub(super) const X_AXIS: f32 = space(8);
const PAD: f32 = space(3);
const COMPACT_PAD: f32 = space(1);
const COMPACT_HEIGHT: f32 = 120.0;
const MIN_PLOT: f32 = space(8);
const LEGEND: f32 = space(6);
pub(super) const ROW: f32 = space(4);
const GAP: f32 = space(2);
const MIN_LABEL_PX: f32 = 32.0;
const FLUID_MAX_LABELS: usize = 12;
const FLUID_COLUMNS: usize = 1024;
const FLUID_PLOT: f32 = 300.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct PlotFrame {
    pub(super) width: Option<f32>,
    pub(super) height: f32,
    pub(super) whole: bool,
    pub(super) titles: u8,
}

impl PlotFrame {
    pub(super) fn padding(self) -> f32 {
        if self.height < COMPACT_HEIGHT {
            COMPACT_PAD
        } else {
            PAD
        }
    }

    fn inner(self, legend: bool) -> f32 {
        let legend = if legend { LEGEND } else { 0.0 };
        let titles = f32::from(self.titles) * (ROW + GAP);
        2.0f32.mul_add(-self.padding(), self.height) - legend - titles
    }

    fn shows_x_axis(self, legend: bool) -> bool {
        self.inner(legend) >= X_AXIS + MIN_PLOT
    }

    fn value_height(self, legend: bool) -> f32 {
        let axis = if self.shows_x_axis(legend) {
            X_AXIS
        } else {
            0.0
        };
        2.0f32.mul_add(-PLOT_PAD, self.inner(legend) - axis)
    }

    pub(super) fn chrome(titles: u8, legend: bool) -> f32 {
        let legend = if legend { LEGEND } else { 0.0 };
        let titles = f32::from(titles) * (ROW + GAP);
        2.0f32.mul_add(PAD + PLOT_PAD, X_AXIS + legend + titles)
    }

    fn plot_width(self) -> f32 {
        self.width.map_or(FLUID_PLOT, |width| {
            2.0f32.mul_add(-(self.padding() + PLOT_PAD), width) - AXIS - GAP
        })
    }

    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "the label budget is a small non-negative count"
    )]
    fn label_stride(self, count: usize) -> usize {
        let budget = self.width.map_or(FLUID_MAX_LABELS, |_| {
            (self.plot_width() / MIN_LABEL_PX).floor().max(1.0) as usize
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
    pub(super) scales: &'a Scales,
}

impl Plot<'_> {
    fn horizontal(&self) -> bool {
        self.mark == ChartMark::Bar && self.scales.orient == Orient::Horizontal
    }

    fn legend(&self) -> bool {
        self.series.iter().any(|item| !item.legend.is_empty())
    }
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
        hues: plot.scales.slices.as_deref(),
    };
    pie::draw(&slices, interact)
}

fn placed<'a>(plot: &Plot<'a>) -> Option<Placed<'a>> {
    let at = plot
        .positions
        .filter(|_| matches!(plot.mark, ChartMark::Line | ChartMark::Area))?;
    Some(Placed {
        at,
        span: ValueSpan::placed(at.iter(), plot.scales.x, plot.scales.time.is_some()),
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

fn area(plot: &Plot<'_>, spans: Spans, placed: Option<Placed<'_>>) -> NodeBuilder {
    let (series, span) = (plot.series, spans.y);
    match plot.mark {
        ChartMark::Bar => bars::bars(&Bars {
            series,
            count: plot.count,
            span,
            stack: plot.stack,
            pad: PLOT_PAD,
            orient: plot.scales.orient,
        }),
        ChartMark::Candle => layers::candles(series, span, plot.frame.columns(), PLOT_PAD),
        ChartMark::Scatter => layers::dots(series, spans, plot.scales.dot, PLOT_PAD),
        _ => layers::traces(
            &Traces {
                series,
                mark: plot.mark,
                stack: plot.stack,
                count: plot.count,
                span,
                columns: plot.frame.columns(),
                placed,
            },
            PLOT_PAD,
        ),
    }
}

fn value_ticks(plot: &Plot<'_>, span: ValueSpan) -> Vec<Tick> {
    let px = if plot.horizontal() {
        plot.frame.plot_width()
    } else {
        plot.frame.value_height(plot.legend())
    };
    ticks::ticks(span, ticks::count(px), plot.frame.whole)
}

fn cartesian(plot: &Plot<'_>, interact: &Interact) -> Vec<NodeBuilder> {
    let (frame, has_legend) = (plot.frame, plot.legend());
    let span = ValueSpan::of(plot.series, plot.mark, plot.stack, plot.scales.y);
    let spans = Spans::of(plot.series, span, plot.scales.x);
    let placed = placed(plot);
    let mut drawn = area(plot, spans, placed);
    if !plot.horizontal() {
        drawn = interact.attach(drawn, plot.total, PLOT_PAD, || tips(plot, spans, placed));
    }
    let x_axis_on = frame.shows_x_axis(has_legend);
    let mut column = container()
        .layout(
            LayoutStyle::flex_column()
                .grow(1.0)
                .min_size(0.0, 0.0)
                .fill_height(),
        )
        .child(drawn);
    if x_axis_on {
        column = column.child(bottom_axis(plot, spans, placed));
    }
    let (left, lead) = left_axis(plot, span, x_axis_on);
    let mut nodes = Vec::new();
    if let Some(name) = side_title(plot) {
        nodes.push(title(name, 0.0, false));
    }
    nodes.push(
        container()
            .layout(
                LayoutStyle::flex_row()
                    .fill_width()
                    .grow(1.0)
                    .min_size(0.0, 0.0)
                    .gap(Space::px(GAP)),
            )
            .child(left)
            .child(column),
    );
    if let Some(name) = foot_title(plot) {
        nodes.push(title(name, lead + GAP, true));
    }
    if has_legend {
        nodes.push(legend(plot.series));
    }
    nodes
}

fn side_title<'a>(plot: &Plot<'a>) -> Option<&'a str> {
    plot.scales.y_title.as_deref()
}

fn foot_title<'a>(plot: &Plot<'a>) -> Option<&'a str> {
    plot.scales.x_title.as_deref()
}

fn left_axis(plot: &Plot<'_>, span: ValueSpan, x_axis_on: bool) -> (NodeBuilder, f32) {
    let bottom = PLOT_PAD + if x_axis_on { X_AXIS } else { 0.0 };
    if plot.horizontal() {
        let shown = plot.labels.get(..plot.count).unwrap_or(plot.labels);
        let width = axis_width(shown.iter().map(String::as_str));
        let gutter = Gutter {
            width,
            top: PLOT_PAD,
            bottom,
        };
        return (category_axis(plot.labels, plot.count, &gutter), width);
    }
    let ticks = value_ticks(plot, span);
    let width = axis_width(ticks.iter().map(|item| item.text.as_str()));
    let gutter = Gutter {
        width,
        top: PLOT_PAD,
        bottom,
    };
    (y_axis(&ticks, &gutter), width)
}

fn bottom_axis(plot: &Plot<'_>, spans: Spans, placed: Option<Placed<'_>>) -> NodeBuilder {
    let (slot_count, stride) = (plot.count, plot.frame.label_stride(plot.count));
    let width = plot.frame.plot_width();
    let whole = plot.frame.whole;
    if plot.horizontal() {
        let marks = fitted(width, |count| ticks::ticks(spans.y, count, whole));
        return x_axis(&marks, width, PLOT_PAD);
    }
    match (plot.mark, placed) {
        (ChartMark::Scatter, _) => {
            let marks = fitted(width, |count| ticks::ticks(spans.x, count, false));
            x_axis(&marks, width, PLOT_PAD)
        }
        (_, Some(placed)) => {
            let marks = fitted(width, |count| {
                plot.scales.time.map_or_else(
                    || ticks::ticks(placed.span, count, false),
                    |origin| clock::ticks(origin, placed.span, count),
                )
            });
            x_axis(&marks, width, PLOT_PAD)
        }
        _ => slots(plot.labels, slot_count, stride),
    }
}

fn fitted(width: f32, make: impl Fn(usize) -> Vec<Tick>) -> Vec<Tick> {
    let mut count = ticks::count(width);
    let mut marks = make(count);
    while count > MIN_TICKS && crowded(&marks, width) {
        count -= 1;
        marks = make(count);
    }
    marks
}

fn slots(labels: &[String], count: usize, stride: usize) -> NodeBuilder {
    let mut row = container().layout(
        LayoutStyle::flex_row()
            .fill_width()
            .height(X_AXIS)
            .shrink(0.0)
            .padding_x(Space::px(PLOT_PAD)),
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
                .flex_basis(Length::Px(0.0))
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
