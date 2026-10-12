use crate::style::strings::StringKey;
use crate::style::theme::ambient::{ActiveTheme, theme_label, theme_text};
use std::ops::Range;
use std::rc::Rc;

use crate::kernel::{AccessibleName, Cx, IntoNode, NodeBuilder, Role, container};
use crate::layout::{AlignItems, JustifyContent, LayoutBuilderExt, LayoutStyle};
use crate::reactive::{Reactive, Signal, derived};
use crate::style::{
    BORDER_WIDTH, PaintTarget, Radius, Style, StyleAtom, StyleSlot, Token, Tone, space,
};

use super::layout_ext::{LayoutOverrides, WidgetLayoutExt};
use crate::kernel::text;
mod axis;
mod decimate;
mod interact;
mod marks;
mod plot;

const DEFAULT_HEIGHT: f32 = 180.0;
const MIN_HEIGHT: f32 = 48.0;
const MIN_WIDTH: f32 = 80.0;
const SUMMARY_VALUES: usize = 64;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ChartMark {
    #[default]
    Bar,
    Line,
    Candle,
}

#[derive(Clone, PartialEq)]
pub(super) struct ChartSeries {
    name: String,
    values: Vec<f32>,
    tone: Tone,
}

pub(super) struct ChartView<'a> {
    name: &'a str,
    values: &'a [f32],
    tone: Tone,
}

struct SeriesSource {
    name: String,
    values: Rc<dyn Reactive<Vec<f32>>>,
    tone: Tone,
}

pub struct ChartBuilder {
    series: Vec<SeriesSource>,
    labels: Vec<String>,
    mark: ChartMark,
    x_range: Option<Signal<Range<f32>>>,
    crosshair: bool,
    whole: bool,
    layout_overrides: LayoutOverrides,
    style_accumulator: Style,
}

#[must_use]
pub fn chart(values: impl Reactive<Vec<f32>>) -> ChartBuilder {
    ChartBuilder {
        series: vec![SeriesSource {
            name: String::new(),
            values: Rc::new(values),
            tone: Tone::Primary,
        }],
        labels: Vec::new(),
        mark: ChartMark::Bar,
        x_range: None,
        crosshair: false,
        whole: false,
        layout_overrides: LayoutOverrides::new(),
        style_accumulator: Style::new(),
    }
}

impl ChartBuilder {
    #[must_use]
    pub fn candles(rows: impl Reactive<Vec<(f32, f32, f32, f32)>>) -> Self {
        let flat = move || {
            rows.read()
                .into_iter()
                .flat_map(<[f32; 4]>::from)
                .collect::<Vec<f32>>()
        };
        let mut builder = chart(flat);
        builder.mark = ChartMark::Candle;
        builder
    }

    #[must_use]
    pub const fn x_range(mut self, range: Signal<Range<f32>>) -> Self {
        self.x_range = Some(range);
        self
    }

    #[must_use]
    pub const fn crosshair(mut self) -> Self {
        self.crosshair = true;
        self
    }

    #[must_use]
    pub const fn whole_numbers(mut self) -> Self {
        self.whole = true;
        self
    }

    #[must_use]
    pub const fn bar(mut self) -> Self {
        self.mark = ChartMark::Bar;
        self
    }

    #[must_use]
    pub const fn line(mut self) -> Self {
        self.mark = ChartMark::Line;
        self
    }

    #[must_use]
    pub fn tone(mut self, tone: Tone) -> Self {
        if let Some(series) = self.series.first_mut() {
            series.tone = tone;
        }
        self
    }

    #[must_use]
    pub fn labels(mut self, labels: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.labels = labels.into_iter().map(Into::into).collect();
        self
    }

    #[must_use]
    pub fn series(
        mut self,
        name: impl Into<String>,
        values: impl Reactive<Vec<f32>>,
        tone: Tone,
    ) -> Self {
        self.series.push(SeriesSource {
            name: name.into(),
            values: Rc::new(values),
            tone,
        });
        self
    }

    fn frame(&mut self) -> plot::Frame {
        let width = self
            .layout_overrides
            .width_hint()
            .map(|px| px.max(MIN_WIDTH));
        let height = self
            .layout_overrides
            .height_hint()
            .map_or(DEFAULT_HEIGHT, |px| px.max(MIN_HEIGHT));
        if let Some(px) = width {
            self.layout_overrides.width(px);
        }
        self.layout_overrides.height(height);
        plot::Frame {
            width,
            height,
            whole: self.whole,
        }
    }

    fn into_node_builder(mut self) -> NodeBuilder {
        let frame = self.frame();
        let sources = std::mem::take(&mut self.series);
        let series = derived(move || {
            sources
                .iter()
                .map(|source| ChartSeries {
                    name: source.name.clone(),
                    values: source.values.read(),
                    tone: source.tone,
                })
                .collect::<Vec<_>>()
        });
        let labels = Rc::new(std::mem::take(&mut self.labels));
        let label_names = Rc::clone(&labels);
        let theme = ActiveTheme::capture();
        let description =
            derived(move || series.with(|series| summary(series, &label_names, theme)));
        let mark = self.mark;
        let interact = interact::Interact::new(self.x_range, self.crosshair);
        let paint = std::mem::replace(&mut self.style_accumulator, Style::new());
        let mut root = container()
            .role(Role::Figure)
            .label(theme_label(StringKey::Chart))
            .mutate(move |target| {
                target.a11y.description = Some(AccessibleName::Derived(description));
            })
            .class([
                Radius::Md.atom(),
                StyleAtom::BorderWidth(BORDER_WIDTH),
                Token::Gray6.border(),
                Token::Surface.bg(),
            ])
            .with(
                self.layout_overrides.apply(
                    LayoutStyle::flex_column()
                        .fill_width()
                        .min_width(0.0)
                        .height(frame.height)
                        .padding(frame.padding())
                        .gap(space(2)),
                ),
            );
        if !paint.is_empty() {
            root = root.style(paint);
        }
        root.children_reactive(move || {
            series.with(|series| content(series, &labels, mark, frame, &interact))
        })
    }
}

fn content(
    series: &[ChartSeries],
    labels: &[String],
    mark: ChartMark,
    frame: plot::Frame,
    interact: &interact::Interact,
) -> Vec<NodeBuilder> {
    let stride = if mark == ChartMark::Candle { 4 } else { 1 };
    let total = series
        .iter()
        .map(|series| series.values.len() / stride)
        .max()
        .unwrap_or(0);
    if total == 0 {
        return vec![empty()];
    }
    let window = interact.window(total);
    let visible = series
        .iter()
        .map(|item| ChartView {
            name: &item.name,
            values: clip(&item.values, window.start * stride, window.end * stride),
            tone: item.tone,
        })
        .collect::<Vec<_>>();
    let shown = clip(labels, window.start, window.end);
    plot::draw(
        &plot::Plot {
            series: &visible,
            labels: shown,
            mark,
            count: window.len(),
            total,
            frame,
        },
        interact,
    )
}

fn clip<T>(items: &[T], start: usize, end: usize) -> &[T] {
    let end = end.min(items.len());
    items.get(start.min(end)..end).unwrap_or_default()
}

fn empty() -> NodeBuilder {
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

fn summary(series: &[ChartSeries], labels: &[String], theme: ActiveTheme) -> String {
    let mut parts = Vec::with_capacity(series.len());
    for (index, item) in series.iter().enumerate() {
        let name = if item.name.is_empty() {
            theme.fill(StringKey::Series, &[&(index + 1)])
        } else {
            item.name.clone()
        };
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

impl IntoNode for ChartBuilder {
    fn build(self, cx: &mut Cx) -> crate::kernel::NodeId {
        self.layout_overrides
            .hooks()
            .finish(self.into_node_builder())
            .build(cx)
    }
}

impl PaintTarget for ChartBuilder {
    fn style_slot(&mut self) -> StyleSlot<'_> {
        StyleSlot(&mut self.style_accumulator)
    }
}

impl WidgetLayoutExt for ChartBuilder {
    fn layout_overrides_mut(&mut self) -> &mut LayoutOverrides {
        &mut self.layout_overrides
    }
}

pub(crate) fn nice_max(max: f32) -> f32 {
    if !max.is_finite() || max <= 0.0 {
        return 1.0;
    }
    let exp = 10f32.powf(max.log10().floor());
    let n = max / exp;
    let nice = [1.0, 2.0, 5.0].into_iter().find(|&step| n <= step);
    nice.unwrap_or(10.0) * exp
}

#[cfg(test)]
#[path = "chart_tests.rs"]
mod tests;

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
