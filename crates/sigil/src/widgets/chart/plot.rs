use std::rc::Rc;

use crate::kernel::{NodeBuilder, container};
use crate::layout::{AlignItems, Dimension, JustifyContent, LayoutBuilderExt, LayoutStyle};
use crate::style::{NodeStyleExt, Radius, Token, space};

use super::axis::{legend, y_axis};
use super::decimate::Candle;
use super::interact::Interact;
use super::{ChartMark, ChartView, marks, nice_max};

pub(super) const AXIS: f32 = 32.0;
const PLOT_PAD: f32 = space(2);
const GROUP_GAP: f32 = space(2);
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
pub(super) struct Frame {
    pub(super) width: Option<f32>,
    pub(super) height: f32,
}

impl Frame {
    pub(super) const fn new(width: Option<f32>, height: f32) -> Self {
        Self { width, height }
    }

    pub(super) fn padding(self) -> f32 {
        if self.height < COMPACT_HEIGHT {
            COMPACT_PAD
        } else {
            PAD
        }
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
    pub(super) mark: ChartMark,
    pub(super) count: usize,
    pub(super) total: usize,
    pub(super) frame: Frame,
}

pub(super) fn draw(plot: &Plot<'_>, interact: &Interact) -> Vec<NodeBuilder> {
    let (series, frame, count) = (plot.series, plot.frame, plot.count);
    let span = Span::of(series, plot.mark);
    let (area, gap) = match plot.mark {
        ChartMark::Bar => (bars(series, count, span), GROUP_GAP),
        ChartMark::Line => (lines(series, count, span, frame.columns()), 0.0),
        ChartMark::Candle => (candles(series, span, frame.columns()), 0.0),
    };
    let area = interact.attach(area, plot.total, PLOT_PAD, || {
        probe_values(series, plot.mark)
    });
    let has_legend = series.iter().any(|item| !item.name.is_empty());
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
        column = column.child(x_axis(plot.labels, count, gap, frame.label_stride(count)));
    }
    let mid_tick = frame.plot_height(has_legend) >= MID_TICK_MIN;
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

fn probe_values(series: &[ChartView<'_>], mark: ChartMark) -> Rc<[f32]> {
    let Some(first) = series.first() else {
        return Rc::from(Vec::new());
    };
    match mark {
        ChartMark::Candle => Candle::rows(first.values)
            .iter()
            .map(|row| row.close)
            .collect(),
        ChartMark::Bar | ChartMark::Line => Rc::from(first.values),
    }
}

#[derive(Clone, Copy)]
pub(super) struct Span {
    pub(super) lo: f32,
    pub(super) hi: f32,
}

impl Span {
    fn of(series: &[ChartView<'_>], mark: ChartMark) -> Self {
        if mark == ChartMark::Candle {
            return Self::tight(series);
        }
        let (mut peak, mut trough) = (0.0_f32, 0.0_f32);
        for item in series {
            for &value in item.values {
                peak = peak.max(value);
                trough = trough.min(value);
            }
        }
        Self {
            lo: if trough < 0.0 {
                -nice_max(-trough)
            } else {
                0.0
            },
            hi: if peak > 0.0 || trough >= 0.0 {
                nice_max(peak)
            } else {
                0.0
            },
        }
    }

    fn tight(series: &[ChartView<'_>]) -> Self {
        let (mut lo, mut hi) = (f32::INFINITY, f32::NEG_INFINITY);
        for item in series {
            for &value in item.values {
                if value.is_finite() {
                    lo = lo.min(value);
                    hi = hi.max(value);
                }
            }
        }
        if !(lo.is_finite() && hi.is_finite()) {
            return Self { lo: 0.0, hi: 1.0 };
        }
        if hi > lo {
            Self { lo, hi }
        } else {
            Self {
                lo: lo - 1.0,
                hi: hi + 1.0,
            }
        }
    }

    pub(super) fn width(self) -> f32 {
        self.hi - self.lo
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

fn bars(series: &[ChartView<'_>], count: usize, span: Span) -> NodeBuilder {
    let mut row = container()
        .layout(
            LayoutStyle::flex_row()
                .fill_width()
                .grow(1.0)
                .min_size(0.0, 0.0)
                .gap(GROUP_GAP)
                .align_items(AlignItems::End)
                .padding(PLOT_PAD),
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
                .gap(2.0)
                .align_items(AlignItems::End),
        );
        for item in series {
            let value = item.values.get(index).copied().unwrap_or(0.0);
            group = group.child(bar_col(span, item.tone, item.name, value));
        }
        row = row.child(group);
    }
    row
}

fn bar_col(span: Span, tone: super::Tone, name: &str, value: f32) -> NodeBuilder {
    let column = container().layout(
        LayoutStyle::flex_column()
            .grow(1.0)
            .flex_basis(Dimension::Px(0.0))
            .min_size(0.0, 0.0)
            .fill_height(),
    );
    if value == 0.0 || !value.is_finite() {
        return column;
    }
    let caption = if name.is_empty() {
        format!("{value:.0}")
    } else {
        format!("{name} {value:.0}")
    };
    let value = value.clamp(span.lo, span.hi);
    let unit = 100.0 / span.width();
    let above = (span.hi - value.max(0.0)) * unit;
    let fill = (value.abs() * unit).max(1.0);
    let below = (value.min(0.0) - span.lo) * unit;
    column
        .child(container().layout(LayoutStyle::block().fill_width().grow(above.max(0.01))))
        .child(
            container()
                .label(caption)
                .layout(
                    LayoutStyle::block()
                        .fill_width()
                        .grow(fill)
                        .min_size(0.0, 2.0),
                )
                .class([Radius::Sm.atom(), tone.fill().bg()])
                .tone(tone),
        )
        .child(container().layout(LayoutStyle::block().fill_width().grow(below.max(0.01))))
}

fn lines(series: &[ChartView<'_>], count: usize, span: Span, columns: usize) -> NodeBuilder {
    let mut layer = container().layout(LayoutStyle::block().fill());
    for item in series {
        if !item.values.is_empty() {
            layer = layer.child(marks::line(item.values, item.tone, count, span, columns));
        }
    }
    plot_area(layer)
}

fn candles(series: &[ChartView<'_>], span: Span, columns: usize) -> NodeBuilder {
    let rows = series
        .first()
        .map(|item| Candle::rows(item.values))
        .unwrap_or_default();
    let mut layer = container().layout(LayoutStyle::block().fill());
    for shape in marks::candles(&rows, span, columns) {
        layer = layer.child(shape);
    }
    plot_area(layer)
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
