use crate::kernel::{Cx, IntoNode, NodeBuilder, Role, container};
use crate::layout::{AlignItems, JustifyContent, LayoutBuilderExt, LayoutStyle};
use crate::style::{BORDER_WIDTH, NodeStyleExt, Radius, StyleAtom, Token, Tone, space};

use super::layout_ext::{LayoutOverrides, WidgetLayoutExt};
use crate::kernel::text;
mod plot;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ChartMark {
    #[default]
    Bar,
    Line,
}

pub(super) struct ChartSeries {
    name: String,
    values: Vec<f32>,
    tone: Tone,
}

pub struct ChartBuilder {
    series: Vec<ChartSeries>,
    labels: Vec<String>,
    mark: ChartMark,
    height: f32,
    width: f32,
    layout_overrides: LayoutOverrides,
}

#[must_use]
pub fn chart(values: impl IntoIterator<Item = f32>) -> ChartBuilder {
    ChartBuilder {
        series: vec![ChartSeries {
            name: String::new(),
            values: values.into_iter().collect(),
            tone: Tone::Primary,
        }],
        labels: Vec::new(),
        mark: ChartMark::Bar,
        height: 180.0,
        width: 280.0,
        layout_overrides: LayoutOverrides::new(),
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
        values: impl IntoIterator<Item = f32>,
        tone: Tone,
    ) -> Self {
        self.series.push(ChartSeries {
            name: name.into(),
            values: values.into_iter().collect(),
            tone,
        });
        self
    }

    #[must_use]
    pub const fn height(mut self, px: f32) -> Self {
        self.height = px.max(48.0);
        self
    }

    #[must_use]
    pub const fn width(mut self, px: f32) -> Self {
        self.width = px.max(80.0);
        self
    }

    fn into_node_builder(self) -> NodeBuilder {
        let count = self
            .series
            .iter()
            .map(|series| series.values.len())
            .max()
            .unwrap_or(0);
        let root = container()
            .role(Role::Figure)
            .label("Chart")
            .description(summary(&self.series, &self.labels))
            .class([
                Radius::Md.atom(),
                StyleAtom::BorderWidth(BORDER_WIDTH),
                Token::Gray6.border(),
                Token::Surface.bg(),
            ])
            .with(
                self.layout_overrides.apply(
                    LayoutStyle::flex_column()
                        .width(self.width)
                        .height(self.height)
                        .padding(space(3))
                        .gap(space(2)),
                ),
            );
        if count == 0 || self.series.iter().all(|s| s.values.is_empty()) {
            return root
                .layout(
                    LayoutStyle::flex_column()
                        .width(self.width)
                        .height(self.height)
                        .align_items(AlignItems::Center)
                        .justify_content(JustifyContent::Center),
                )
                .child(
                    text("No data")
                        .nowrap()
                        .class("text-sm")
                        .class(Token::OnSurfaceMuted.text()),
                );
        }
        plot::draw(root, &self.series, &self.labels, self.mark, count)
    }
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
        self.into_node_builder().build(cx)
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
