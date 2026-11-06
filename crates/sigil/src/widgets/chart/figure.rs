use crate::kernel::{NodeBuilder, Role, container};
use crate::layout::LayoutStyle;
use crate::style::strings::StringKey;
use crate::style::theme::ambient::{ActiveTheme, theme_label};
use crate::style::{BORDER_WIDTH, Radius, StyleAtom, Token, space};

use super::scales::Scales;
use super::{ChartMark, ChartSeries, ChartView, Stack, interact, plot, summary};

pub(super) struct Look {
    pub(super) mark: ChartMark,
    pub(super) stack: Stack,
    pub(super) frame: plot::PlotFrame,
    pub(super) theme: ActiveTheme,
    pub(super) scales: Scales,
}

pub(super) fn frame(frame: plot::PlotFrame) -> NodeBuilder {
    container()
        .role(Role::Figure)
        .label(theme_label(StringKey::Chart))
        .class([
            Radius::Md.atom(),
            StyleAtom::BorderWidth(BORDER_WIDTH),
            Token::Gray6.border(),
            Token::Surface.bg(),
        ])
        .with(
            LayoutStyle::flex_column()
                .fill_width()
                .min_width(0.0)
                .height(frame.height)
                .padding(frame.padding())
                .gap(space(2)),
        )
}

pub(super) fn content(
    series: &[ChartSeries],
    labels: &[String],
    positions: Option<&[f32]>,
    look: &Look,
    interact: &interact::Interact,
) -> Vec<NodeBuilder> {
    let stride = look.mark.stride();
    let total = series
        .iter()
        .map(|series| series.values.len() / stride)
        .max()
        .unwrap_or(0);
    if total == 0 {
        return vec![summary::empty()];
    }
    let window = interact.window(total);
    let many = series.len() > 1;
    let visible = series
        .iter()
        .enumerate()
        .map(|(index, item)| ChartView {
            name: &item.name,
            legend: if many || !item.name.is_empty() {
                summary::series_name(&item.name, index, look.theme)
            } else {
                String::new()
            },
            values: clip(&item.values, window.start * stride, window.end * stride),
            hue: item.hue,
        })
        .collect::<Vec<_>>();
    let shown = clip(labels, window.start, window.end);
    plot::draw(
        &plot::Plot {
            series: &visible,
            labels: shown,
            positions: positions.map(|at| clip(at, window.start, window.end)),
            mark: look.mark,
            stack: look.stack,
            count: window.len(),
            total,
            frame: look.frame,
            scales: &look.scales,
        },
        interact,
    )
}

fn clip<T>(items: &[T], start: usize, end: usize) -> &[T] {
    let end = end.min(items.len());
    items.get(start.min(end)..end).unwrap_or_default()
}
