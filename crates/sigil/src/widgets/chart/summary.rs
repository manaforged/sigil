use crate::kernel::{NodeBuilder, container, text};
use crate::layout::{AlignItems, JustifyContent, LayoutStyle};
use crate::style::Token;
use crate::style::strings::StringKey;
use crate::style::theme::ambient::{ActiveTheme, theme_text};
use crate::widgets::Widget;

use super::ChartSeries;

const SUMMARY_VALUES: usize = 64;

pub(super) fn empty() -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_column()
                .fill_width()
                .grow(1.0)
                .align_items(AlignItems::Center)
                .justify_content(JustifyContent::Center),
        )
        .child(
            text(theme_text(StringKey::NoData))
                .nowrap()
                .class("text-sm")
                .class(Token::OnSurfaceMuted.text()),
        )
}

pub(super) fn series_name(name: &str, index: usize, theme: ActiveTheme) -> String {
    if name.is_empty() {
        theme.fill(StringKey::Series, &[&(index + 1)])
    } else {
        name.to_owned()
    }
}

pub(super) fn summary(series: &[ChartSeries], labels: &[String], theme: ActiveTheme) -> String {
    let mut parts = Vec::with_capacity(series.len());
    for (index, item) in series.iter().enumerate() {
        let name = series_name(&item.name, index, theme);
        if item.values.len() > SUMMARY_VALUES {
            parts.push(span_summary(&name, &item.values, theme));
            continue;
        }
        let values = item
            .values
            .iter()
            .enumerate()
            .map(|(at, value)| {
                labels
                    .get(at)
                    .map_or_else(|| format!("{value}"), |label| format!("{label} {value}"))
            })
            .collect::<Vec<_>>()
            .join(", ");
        parts.push(format!("{name}: {values}"));
    }
    parts.join("; ")
}

fn span_summary(name: &str, values: &[f32], theme: ActiveTheme) -> String {
    let (mut low, mut high) = (f32::INFINITY, f32::NEG_INFINITY);
    for &value in values {
        if value.is_finite() {
            low = low.min(value);
            high = high.max(value);
        }
    }
    theme.fill(
        StringKey::ChartSummary,
        &[&name, &values.len(), &low, &high],
    )
}
