use crate::kernel::{IntoNode, NodeBuilder, container, text};
use crate::layout::Space;
use crate::layout::{AlignItems, JustifyContent, LayoutStyle, Length, Position};
use crate::style::{Radius, Tone};
use crate::widgets::Widget;

use super::ChartView;
use super::plot::{AXIS, ROW, X_AXIS};
use super::ticks::Tick;

const SIGNIFICANT: f32 = 3.0;
const MAX_DECIMALS: f32 = 12.0;
const WHOLE_FROM: f32 = 100.0;
const CHAR_PX: f32 = 6.5;
const LABEL_GAP: f32 = 6.0;
const TICK_H: f32 = 16.0;
const TICK_W: f32 = 120.0;
const AXIS_PAD: f32 = 6.0;
const MAX_AXIS: f32 = 200.0;

#[allow(
    clippy::cast_precision_loss,
    reason = "a label length is a small character count"
)]
pub(super) fn text_px(label: &str) -> f32 {
    label.chars().count() as f32 * CHAR_PX
}

pub(super) fn axis_width<'a>(labels: impl Iterator<Item = &'a str>) -> f32 {
    labels
        .map(|label| text_px(label) + AXIS_PAD)
        .fold(AXIS, f32::max)
        .min(MAX_AXIS)
}

pub(super) struct Gutter {
    pub(super) width: f32,
    pub(super) top: f32,
    pub(super) bottom: f32,
}

impl Gutter {
    fn column(&self) -> NodeBuilder {
        container().layout(
            LayoutStyle::flex_column()
                .width(self.width)
                .shrink(0.0)
                .fill_height()
                .padding_top(Space::px(self.top))
                .padding_bottom(Space::px(self.bottom)),
        )
    }
}

fn y_label(item: &Tick) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_row()
                .position(Position::Absolute)
                .inset_left(0.0)
                .inset_top(Length::Percent(1.0 - item.at))
                .margin_top(Space::px(-TICK_H / 2.0))
                .fill_width()
                .height(TICK_H)
                .justify_content(JustifyContent::End)
                .align_items(AlignItems::Center),
        )
        .child(tick(item.text.clone()))
}

pub(super) fn y_axis(ticks: &[Tick], gutter: &Gutter) -> NodeBuilder {
    let layer = container().layout(
        LayoutStyle::block()
            .fill_width()
            .grow(1.0)
            .min_size(0.0, 0.0),
    );
    gutter
        .column()
        .child(ticks.iter().map(y_label).fold(layer, NodeBuilder::child))
}

pub(super) fn crowded(ticks: &[Tick], width_px: f32) -> bool {
    let kept = ticks.iter().collect::<Vec<_>>();
    overlaps(&kept, width_px)
}

fn overlaps(kept: &[&Tick], width_px: f32) -> bool {
    kept.windows(2).any(|pair| {
        let [left, right] = pair else {
            return false;
        };
        let room = (right.at - left.at).abs() * width_px;
        room < f32::midpoint(text_px(&left.text), text_px(&right.text)) + LABEL_GAP
    })
}

fn parity(ticks: &[Tick], width_px: f32) -> Vec<&Tick> {
    let mut kept = ticks.iter().collect::<Vec<_>>();
    while kept.len() > 1 && overlaps(&kept, width_px) {
        kept = kept.into_iter().step_by(2).collect();
    }
    kept
}

fn x_label(item: &Tick) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_row()
                .position(Position::Absolute)
                .inset_top(0.0)
                .inset_left(Length::Percent(item.at))
                .margin_left(Space::px(-TICK_W / 2.0))
                .width(TICK_W)
                .height(TICK_H)
                .justify_content(JustifyContent::Center),
        )
        .child(tick(item.text.clone()))
}

pub(super) fn x_axis(ticks: &[Tick], width_px: f32, pad: f32) -> NodeBuilder {
    let layer = parity(ticks, width_px).into_iter().map(x_label).fold(
        container().layout(LayoutStyle::block().fill()),
        NodeBuilder::child,
    );
    container()
        .layout(
            LayoutStyle::block()
                .fill_width()
                .height(X_AXIS)
                .shrink(0.0)
                .padding_x(Space::px(pad)),
        )
        .child(layer)
}

pub(super) fn category_axis(labels: &[String], count: usize, gutter: &Gutter) -> NodeBuilder {
    (0..count).fold(gutter.column(), |column, index| {
        let name = labels
            .get(index)
            .cloned()
            .unwrap_or_else(|| (index + 1).to_string());
        column.child(
            container()
                .layout(
                    LayoutStyle::flex_row()
                        .grow(1.0)
                        .flex_basis(Length::Px(0.0))
                        .min_size(0.0, 0.0)
                        .justify_content(JustifyContent::End)
                        .align_items(AlignItems::Center),
                )
                .child(tick(name)),
        )
    })
}

pub(super) fn title(name: &str, lead: f32, centered: bool) -> NodeBuilder {
    let justify = if centered {
        JustifyContent::Center
    } else {
        JustifyContent::Start
    };
    container()
        .layout(
            LayoutStyle::flex_row()
                .fill_width()
                .height(ROW)
                .shrink(0.0)
                .padding_left(Space::px(lead))
                .justify_content(justify),
        )
        .child(
            text(name.to_owned())
                .nowrap()
                .class("text-xs")
                .class("font-bold")
                .class(Tone::Neutral.step(12).text()),
        )
}

fn tick(value: String) -> impl IntoNode + 'static {
    text(value)
        .nowrap()
        .class("text-xs")
        .class(Tone::Neutral.step(11).text())
}

pub(super) fn legend(series: &[ChartView<'_>]) -> NodeBuilder {
    let mut row = container().layout(
        LayoutStyle::flex_row()
            .height(ROW)
            .shrink(0.0)
            .gap(Space::Md)
            .align_items(AlignItems::Center),
    );
    for item in series.iter().filter(|item| !item.legend.is_empty()) {
        row = row.child(
            container()
                .layout(
                    LayoutStyle::flex_row()
                        .gap(Space::px(6.0))
                        .align_items(AlignItems::Center),
                )
                .child(
                    container()
                        .layout(LayoutStyle::block().fixed_size(8.0, 8.0))
                        .class([Radius::Sm.atom(), item.hue.background()]),
                )
                .child(
                    text(item.legend.clone())
                        .nowrap()
                        .class("text-xs")
                        .class(Tone::Neutral.step(12).text()),
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
