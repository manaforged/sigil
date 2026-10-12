use crate::kernel::{IntoNode, NodeBuilder, container, text};
use crate::layout::{AlignItems, JustifyContent, LayoutBuilderExt, LayoutStyle};
use crate::style::{Radius, Token, space};

use super::ChartView;
use super::plot::{AXIS, Span, X_AXIS};

pub(super) fn y_axis(span: Span, x_axis_on: bool, mid_tick: bool) -> NodeBuilder {
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

pub(super) fn legend(series: &[ChartView<'_>]) -> NodeBuilder {
    let mut row = container().layout(LayoutStyle::flex_row().gap(space(3)));
    for item in series.iter().filter(|item| !item.name.is_empty()) {
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
                    text(item.name.to_owned())
                        .nowrap()
                        .class("text-xs")
                        .class(Token::Gray12.text()),
                ),
        );
    }
    row
}

pub(super) fn format_tick(value: f32) -> String {
    if value >= 100.0 || (value - value.round()).abs() < 0.05 {
        format!("{value:.0}")
    } else {
        format!("{value:.1}")
    }
}
