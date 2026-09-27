use crate::kernel::{NodeBuilder, container};
use crate::layout::prelude::{
    AlignItems, Dimension, JustifyContent, LayoutBuilderExt, LayoutStyle,
};
use crate::style::{NodeStyleExt, Radius, Token, space};

use super::super::shared::label;
use super::super::vector::path;
use super::{ChartMark, ChartSeries, nice_max, tone_fill};

const AXIS: f32 = 32.0;
const PLOT_PAD: f32 = space(2);
const GROUP_GAP: f32 = space(2);
const X_AXIS: f32 = space(8);

pub(super) fn draw(
    root: NodeBuilder,
    series: &[ChartSeries],
    labels: &[String],
    mark: ChartMark,
    count: usize,
) -> NodeBuilder {
    let peak = series
        .iter()
        .flat_map(|item| item.values.iter().copied())
        .fold(0.0_f32, f32::max);
    let max = nice_max(peak);
    let plot = match mark {
        ChartMark::Bar => bars(series, count, max),
        ChartMark::Line => lines(series, count, max),
    };
    let mut node = root.child(
        container()
            .layout(
                LayoutStyle::flex_row()
                    .fill_width()
                    .grow(1.0)
                    .min_size(0.0, 0.0)
                    .gap(space(2)),
            )
            .child(y_axis(max))
            .child(
                container()
                    .layout(
                        LayoutStyle::flex_column()
                            .grow(1.0)
                            .min_size(0.0, 0.0)
                            .fill_height(),
                    )
                    .child(plot)
                    .child(x_axis(labels, count)),
            ),
    );
    if series.iter().any(|item| !item.name.is_empty()) {
        node = node.child(legend(series));
    }
    node
}

fn y_axis(max: f32) -> NodeBuilder {
    let ticks = container()
        .layout(
            LayoutStyle::flex_column()
                .fill_width()
                .grow(1.0)
                .align_items(AlignItems::End)
                .justify_content(JustifyContent::SpaceBetween),
        )
        .child(
            label(format_tick(max))
                .class("text-xs")
                .class(Token::Gray11.text()),
        )
        .child(
            label(format_tick(max * 0.5))
                .class("text-xs")
                .class(Token::Gray11.text()),
        )
        .child(label("0").class("text-xs").class(Token::Gray11.text()));
    container()
        .layout(
            LayoutStyle::flex_column()
                .width(AXIS)
                .fill_height()
                .padding_bottom(X_AXIS),
        )
        .child(ticks)
}

fn x_axis(labels: &[String], count: usize) -> NodeBuilder {
    let mut row = container().layout(
        LayoutStyle::flex_row()
            .fill_width()
            .height(X_AXIS)
            .shrink(0.0)
            .padding_x(PLOT_PAD)
            .gap(GROUP_GAP),
    );
    for index in 0..count {
        let text = labels
            .get(index)
            .cloned()
            .unwrap_or_else(|| (index + 1).to_string());
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
                    label(text)
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
                .class([Radius::Sm.atom(), tone_fill(tone).bg()])
                .tone(tone),
        )
}

fn lines(series: &[ChartSeries], count: usize, max: f32) -> NodeBuilder {
    const W: u32 = 400;
    const H: u32 = 200;
    const W_F: f32 = 400.0;
    const H_F: f32 = 200.0;
    let mut layer = container()
        .layout(LayoutStyle::block().fill())
        .class("border-b")
        .class(Token::Gray6.border());
    for item in series {
        if item.values.is_empty() {
            continue;
        }
        let last = u32::try_from(count - 1).unwrap_or(u32::MAX);
        let mut stroke = path(W, H);
        for (point, value) in item.values.iter().take(count).enumerate() {
            #[allow(
                clippy::cast_precision_loss,
                reason = "pixel and char-count magnitudes are bounded; sub-unit precision is irrelevant at render scale"
            )]
            let x = if count <= 1 {
                W_F * 0.5
            } else {
                (point as f32 / last as f32).mul_add(W_F - 8.0, 4.0)
            };
            let t = ((*value).max(0.0) / max).clamp(0.0, 1.0);
            let y = (1.0 - t).mul_add(H_F - 8.0, 4.0);
            stroke = if point == 0 {
                stroke.move_to(x, y)
            } else {
                stroke.line_to(x, y)
            };
        }
        layer = layer.child(
            stroke
                .stroke(tone_fill(item.tone), 2.0)
                .layout(LayoutStyle::block().fill()),
        );
    }
    layer
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
                        .class([Radius::Sm.atom(), tone_fill(item.tone).bg()]),
                )
                .child(
                    label(item.name.clone())
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
