use std::rc::Rc;

use crate::kernel::{AccessibleName, Cx, IntoNode, NodeBuilder, Role, container};
use crate::layout::{AlignItems, JustifyContent, LayoutBuilderExt, LayoutStyle};
use crate::reactive::{Reactive, derived};
use crate::style::{
    BORDER_WIDTH, PaintTarget, Radius, Style, StyleAtom, StyleSlot, Token, Tone, space,
};

use super::layout_ext::{LayoutOverrides, WidgetLayoutExt};
use crate::kernel::text;
mod plot;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ChartMark {
    #[default]
    Bar,
    Line,
}

#[derive(Clone, PartialEq)]
pub(super) struct ChartSeries {
    name: String,
    values: Vec<f32>,
    tone: Tone,
}

struct SeriesSource {
    name: String,
    values: Rc<dyn Reactive<Vec<f32>>>,
    tone: Tone,
}

impl SeriesSource {
    fn read(&self) -> ChartSeries {
        ChartSeries {
            name: self.name.clone(),
            values: self.values.read(),
            tone: self.tone,
        }
    }
}

pub struct ChartBuilder {
    series: Vec<SeriesSource>,
    labels: Vec<String>,
    mark: ChartMark,
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
        layout_overrides: LayoutOverrides::new(),
        style_accumulator: Style::new(),
    }
}

impl ChartBuilder {
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

    fn frame_size(&mut self) -> (f32, f32) {
        let width = self
            .layout_overrides
            .width_hint()
            .map_or(280.0, |px| px.max(80.0));
        let height = self
            .layout_overrides
            .height_hint()
            .map_or(180.0, |px| px.max(48.0));
        self.layout_overrides.width(width);
        self.layout_overrides.height(height);
        (width, height)
    }

    fn into_node_builder(mut self) -> NodeBuilder {
        let (width, height) = self.frame_size();
        let sources = std::mem::take(&mut self.series);
        let series = derived(move || sources.iter().map(SeriesSource::read).collect::<Vec<_>>());
        let labels = Rc::new(std::mem::take(&mut self.labels));
        let label_names = Rc::clone(&labels);
        let description = derived(move || series.with(|series| summary(series, &label_names)));
        let mark = self.mark;
        let paint = std::mem::replace(&mut self.style_accumulator, Style::new());
        let mut root = container()
            .role(Role::Figure)
            .label("Chart")
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
                        .width(width)
                        .height(height)
                        .padding(space(3))
                        .gap(space(2)),
                ),
            );
        if !paint.is_empty() {
            root = root.style(paint);
        }
        root.children_reactive(move || series.with(|series| content(series, &labels, mark)))
    }
}

fn content(series: &[ChartSeries], labels: &[String], mark: ChartMark) -> Vec<NodeBuilder> {
    let count = series
        .iter()
        .map(|series| series.values.len())
        .max()
        .unwrap_or(0);
    if count == 0 {
        return vec![
            container()
                .layout(
                    LayoutStyle::flex_column()
                        .fill_width()
                        .grow(1.0)
                        .align_items(AlignItems::Center)
                        .justify_content(JustifyContent::Center),
                )
                .child(
                    text("No data")
                        .nowrap()
                        .class("text-sm")
                        .class(Token::OnSurfaceMuted.text()),
                ),
        ];
    }
    plot::draw(series, labels, mark, count)
}

fn summary(series: &[ChartSeries], labels: &[String]) -> String {
    let mut parts = Vec::with_capacity(series.len());
    for (index, item) in series.iter().enumerate() {
        let name = if item.name.is_empty() {
            format!("Series {}", index + 1)
        } else {
            item.name.clone()
        };
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

const SOLID_STEP: u8 = 11;
const SHADE_STEP: u8 = 4;

pub(crate) fn tone_solid(tone: Tone) -> Token {
    Token::scale(tone.scale_role(), SOLID_STEP).unwrap_or(Token::Gray11)
}

pub(crate) fn tone_shade(tone: Tone) -> Token {
    Token::scale(tone.scale_role(), SHADE_STEP).unwrap_or(Token::Gray4)
}

pub(crate) const fn tone_fill(tone: Tone) -> Token {
    match tone {
        Tone::Neutral => Token::Gray9,
        Tone::Primary => Token::Accent9,
        Tone::Info => Token::Info9,
        Tone::Success => Token::Success9,
        Tone::Warn => Token::Warn9,
        Tone::Error => Token::Error9,
    }
}

pub(crate) fn nice_max(max: f32) -> f32 {
    if !max.is_finite() || max <= 0.0 {
        return 1.0;
    }
    let exp = 10f32.powf(max.log10().floor());
    let n = max / exp;
    let nice = if n <= 1.0 {
        1.0
    } else if n <= 2.0 {
        2.0
    } else if n <= 5.0 {
        5.0
    } else {
        10.0
    };
    nice * exp
}

#[cfg(test)]
#[path = "chart_tests.rs"]
mod tests;
