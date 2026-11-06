use crate::kernel::{IntoNode, NodeBuilder, container, text};
use crate::layout::{AlignItems, Dimension, JustifyContent, LayoutStyle, Position};
use crate::style::{Radius, Token, space};
use crate::widgets::Widget;

use super::ChartView;
use super::marks::Placed;
use super::plot::{AXIS, X_AXIS};
use super::span::ValueSpan;

const SIGNIFICANT: f32 = 3.0;
const MAX_DECIMALS: f32 = 12.0;
const WHOLE_FROM: f32 = 100.0;

pub(super) fn y_axis(span: ValueSpan, x_axis_on: bool, mid_tick: bool) -> NodeBuilder {
    let mut ticks = container()
        .layout(
            LayoutStyle::flex_column()
                .fill_width()
                .grow(1.0)
                .align_items(AlignItems::End)
                .justify_content(JustifyContent::SpaceBetween),
        )
        .child(tick(format_tick(span.hi)));
    if mid_tick {
        ticks = ticks.child(tick(format_tick(span.lo.midpoint(span.hi))));
    }
    ticks = ticks.child(tick(format_tick(span.lo)));
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

#[allow(
    clippy::cast_precision_loss,
    reason = "a label stride and count are small"
)]
pub(super) fn placed_axis(
    labels: &[String],
    placed: Placed<'_>,
    stride: usize,
    pad: f32,
) -> NodeBuilder {
    let stride = stride.max(1);
    let half = 0.5 * stride as f32 / placed.at.len().max(1) as f32;
    let layer = (0..placed.at.len()).step_by(stride).fold(
        container().layout(LayoutStyle::block().fill()),
        |layer, index| {
            let name = labels
                .get(index)
                .cloned()
                .unwrap_or_else(|| (index + 1).to_string());
            layer.child(placed_label(name, placed.fraction(index), half))
        },
    );
    container()
        .layout(
            LayoutStyle::block()
                .fill_width()
                .height(X_AXIS)
                .shrink(0.0)
                .padding_x(pad),
        )
        .child(layer)
}

fn placed_label(name: String, fraction: f32, half: f32) -> NodeBuilder {
    let start = (fraction - half).clamp(0.0, 1.0);
    let end = (fraction + half).clamp(start, 1.0);
    container()
        .layout(
            LayoutStyle::flex_row()
                .position(Position::Absolute)
                .inset_top(0.0)
                .inset_left(0.0)
                .fill(),
        )
        .child(share(start))
        .child(
            container()
                .layout(
                    LayoutStyle::flex_row()
                        .grow(end - start)
                        .flex_basis(Dimension::Px(0.0))
                        .min_size(0.0, 0.0)
                        .justify_content(JustifyContent::Center),
                )
                .child(tick(name)),
        )
        .child(share(1.0 - end))
}

fn share(grow: f32) -> NodeBuilder {
    container().layout(
        LayoutStyle::block()
            .grow(grow)
            .flex_basis(Dimension::Px(0.0))
            .min_size(0.0, 0.0),
    )
}

pub(super) fn legend(series: &[ChartView<'_>]) -> NodeBuilder {
    let mut row = container().layout(LayoutStyle::flex_row().gap(space(3)));
    for item in series.iter().filter(|item| !item.legend.is_empty()) {
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
                        .class([Radius::Sm.atom(), item.hue.token().bg()]),
                )
                .child(
                    text(item.legend.clone())
                        .nowrap()
                        .class("text-xs")
                        .class(Token::Gray12.text()),
                ),
        );
    }
    row
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the decimal count is clamped to a small non-negative number"
)]
pub(super) fn format_tick(value: f32) -> String {
    if value == 0.0 {
        return "0".to_owned();
    }
    if !value.is_finite() || value.fract() == 0.0 || value.abs() >= WHOLE_FROM {
        return format!("{value:.0}");
    }
    let magnitude = value.abs().log10().floor();
    let decimals = (SIGNIFICANT - 1.0 - magnitude).clamp(0.0, MAX_DECIMALS) as usize;
    let fixed = format!("{value:.decimals$}");
    let trimmed = fixed.trim_end_matches('0').trim_end_matches('.');
    let shown = if trimmed == "-0" { "0" } else { trimmed };
    shown.to_owned()
}
