use crate::kernel::{IntoNode, NodeBuilder, container};
use crate::layout::{
    AlignItems, Dimension, JustifyContent, LayoutBuilderExt, LayoutStyle, Position,
};
use crate::style::{NodeStyleExt, Radius, Token, space};

use super::super::vector::{PathNode, path};
use super::{ChartMark, ChartSeries, nice_max};
use crate::kernel::text;

const AXIS: f32 = 32.0;
const PLOT_PAD: f32 = space(2);
const GROUP_GAP: f32 = space(2);
const X_AXIS: f32 = space(8);
const PAD: f32 = space(3);
const COMPACT_PAD: f32 = space(1);
const COMPACT_HEIGHT: f32 = 120.0;
const MIN_PLOT: f32 = space(8);
const MID_TICK_MIN: f32 = space(12);
const LEGEND: f32 = space(6);
const MIN_LABEL_PX: f32 = 32.0;
const FLUID_MAX_LABELS: usize = 12;

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
}

pub(super) fn draw(
    series: &[ChartSeries],
    labels: &[String],
    mark: ChartMark,
    count: usize,
    frame: Frame,
) -> Vec<NodeBuilder> {
    let peak = series
        .iter()
        .flat_map(|item| item.values.iter().copied())
        .fold(0.0_f32, f32::max);
    let max = nice_max(peak);
    let (plot, gap) = match mark {
        ChartMark::Bar => (bars(series, count, max), GROUP_GAP),
        ChartMark::Line => (lines(series, count, max), 0.0),
    };
    let has_legend = series.iter().any(|item| !item.name.is_empty());
    let x_axis_on = frame.shows_x_axis(has_legend);
    let mut column = container()
        .layout(
            LayoutStyle::flex_column()
                .grow(1.0)
                .min_size(0.0, 0.0)
                .fill_height(),
        )
        .child(plot);
    if x_axis_on {
        column = column.child(x_axis(labels, count, gap, frame.label_stride(count)));
    }
    let mut nodes = vec![
        container()
            .layout(
                LayoutStyle::flex_row()
                    .fill_width()
                    .grow(1.0)
                    .min_size(0.0, 0.0)
                    .gap(space(2)),
            )
            .child(y_axis(
                max,
                x_axis_on,
                frame.plot_height(has_legend) >= MID_TICK_MIN,
            ))
            .child(column),
    ];
    if has_legend {
        nodes.push(legend(series));
    }
    nodes
}

fn y_axis(max: f32, x_axis_on: bool, mid_tick: bool) -> NodeBuilder {
    let mut ticks = container()
        .layout(
            LayoutStyle::flex_column()
                .fill_width()
                .grow(1.0)
                .align_items(AlignItems::End)
                .justify_content(JustifyContent::SpaceBetween),
        )
        .child(tick(format_tick(max)));
    if mid_tick {
        ticks = ticks.child(tick(format_tick(max * 0.5)));
    }
    ticks = ticks.child(tick(String::from("0")));
    container()
        .layout(
            LayoutStyle::flex_column()
                .width(AXIS)
                .fill_height()
                .padding_bottom(if x_axis_on { X_AXIS } else { 0.0 }),
        )
        .child(ticks)
}

fn tick(value: String) -> impl IntoNode + 'static {
    text(value)
        .nowrap()
        .class("text-xs")
        .class(Token::Gray11.text())
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
    for index in 0..count {
        let text = if index % stride == 0 {
            labels
                .get(index)
                .cloned()
                .unwrap_or_else(|| (index + 1).to_string())
        } else {
            String::new()
        };
        row = row.child(
            container()
                .layout(
                    LayoutStyle::flex_row()
                        .grow(1.0)
                        .flex_basis(Dimension::Px(0.0))
                        .min_size(0.0, 0.0)
                        .justify_content(JustifyContent::Center),
                )
                .child(
                    crate::kernel::text(text)
                        .wrap()
                        .class("text-xs")
                        .class(Token::Gray11.text()),
                ),
        );
    }
    row
}

fn bars(series: &[ChartSeries], count: usize, max: f32) -> NodeBuilder {
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
            let value = item.values.get(index).copied().unwrap_or(0.0).max(0.0);
            let pct = ((value / max) * 100.0).clamp(0.0, 100.0);
            group = group.child(bar_col(pct, item.tone, &item.name, value));
        }
        row = row.child(group);
    }
    row
}

fn bar_col(pct: f32, tone: super::Tone, name: &str, value: f32) -> NodeBuilder {
    if value <= 0.0 {
        return container().layout(LayoutStyle::block().grow(1.0).fill_height());
    }
    let caption = if name.is_empty() {
        format!("{value:.0}")
    } else {
        format!("{name} {value:.0}")
    };
    let fill = pct.max(1.0);
    let rest = (100.0 - fill).max(0.01);
    container()
        .layout(
            LayoutStyle::flex_column()
                .grow(1.0)
                .flex_basis(Dimension::Px(0.0))
                .min_size(0.0, 0.0)
                .fill_height(),
        )
        .child(container().layout(LayoutStyle::block().fill_width().grow(rest)))
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
}

fn lines(series: &[ChartSeries], count: usize, max: f32) -> NodeBuilder {
    let mut layer = container().layout(LayoutStyle::block().fill());
    for item in series {
        if item.values.is_empty() {
            continue;
        }
        layer = layer.child(
            line_path(item, count, max).layout(
                LayoutStyle::block()
                    .position(Position::Absolute)
                    .inset_top(0.0)
                    .inset_left(0.0)
                    .fill(),
            ),
        );
    }
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

#[allow(
    clippy::cast_precision_loss,
    reason = "point counts are small; sub-unit precision is irrelevant at render scale"
)]
fn line_path(item: &ChartSeries, count: usize, max: f32) -> PathNode {
    const W: u32 = 400;
    const H: u32 = 200;
    const W_F: f32 = 400.0;
    const H_F: f32 = 200.0;
    let columns = count as f32;
    let mut stroke = path(W, H);
    for (point, value) in item.values.iter().take(count).enumerate() {
        let x = W_F * (point as f32 + 0.5) / columns;
        let t = ((*value).max(0.0) / max).clamp(0.0, 1.0);
        let y = (1.0 - t).mul_add(H_F - 8.0, 4.0);
        stroke = if point == 0 {
            stroke.move_to(x, y)
        } else {
            stroke.line_to(x, y)
        };
    }
    stroke.stroke(item.tone.fill(), 2.0).non_scaling_stroke()
}

fn legend(series: &[ChartSeries]) -> NodeBuilder {
    let mut row = container().layout(LayoutStyle::flex_row().gap(space(3)));
    for item in series {
        if item.name.is_empty() {
            continue;
        }
        row = row.child(
            container()
                .layout(
                    LayoutStyle::flex_row()
                        .gap(6.0)
                        .align_items(AlignItems::Center),
                )
                .child(
                    container()
                        .layout(LayoutStyle::block().fixed_size(8.0, 8.0))
                        .class([Radius::Sm.atom(), item.tone.fill().bg()]),
                )
                .child(
                    text(item.name.clone())
                        .nowrap()
                        .class("text-xs")
                        .class(Token::Gray12.text()),
                ),
        );
    }
    row
}

fn format_tick(value: f32) -> String {
    if value >= 100.0 || (value - value.round()).abs() < 0.05 {
        format!("{value:.0}")
    } else {
        format!("{value:.1}")
    }
}
