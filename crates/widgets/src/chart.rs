//! Tone-native chart — bar and line marks (DR-057 D2).
//!
//! Data is a list of numbers. Style follows [`Tone`]. Placement is
//! `.width` / `.height`. Open the hood with extra series.

use sigil_kernel::{Cx, IntoNode, NodeBuilder, Role, container};
use sigil_layout::prelude::{AlignItems, JustifyContent, LayoutBuilderExt, LayoutStyle};
use sigil_style::{NodeStyleExt, Theme, Tone};

use super::layout_ext::{LayoutOverrides, WidgetLayoutExt};
use super::shared::label;
mod plot;

/// How values are drawn.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ChartMark {
    /// Vertical bars, one group per index.
    #[default]
    Bar,
    /// Polyline through each index.
    Line,
}

/// One named series.
pub(super) struct ChartSeries {
    name: String,
    values: Vec<f32>,
    tone: Tone,
}

/// Builder returned by [`chart`].
pub struct ChartBuilder {
    series: Vec<ChartSeries>,
    labels: Vec<String>,
    mark: ChartMark,
    height: f32,
    width: f32,
    layout_overrides: LayoutOverrides,
}

/// Chart from one series of values. Defaults to bars and
/// [`Tone::Primary`].
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
    /// Draw vertical bars.
    #[must_use]
    pub const fn bar(mut self) -> Self {
        self.mark = ChartMark::Bar;
        self
    }

    /// Draw a polyline.
    #[must_use]
    pub const fn line(mut self) -> Self {
        self.mark = ChartMark::Line;
        self
    }

    /// Retint the first series.
    #[must_use]
    pub fn tone(mut self, tone: Tone) -> Self {
        if let Some(series) = self.series.first_mut() {
            series.tone = tone;
        }
        self
    }

    /// Bottom labels, one per index.
    #[must_use]
    pub fn labels(mut self, labels: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.labels = labels.into_iter().map(Into::into).collect();
        self
    }

    /// Add another series.
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

    /// Plot height in logical pixels.
    #[must_use]
    pub const fn height(mut self, px: f32) -> Self {
        self.height = px.max(48.0);
        self
    }

    /// Plot width in logical pixels.
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
            .role(Role::GenericContainer)
            .label("Chart")
            .class("rounded-md border border-gray-6 bg-surface")
            .layout(
                self.layout_overrides.apply(
                    LayoutStyle::flex_column()
                        .width(self.width)
                        .height(self.height)
                        .padding(12.0)
                        .gap(8.0),
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
                .child(label("No data").class("text-sm text-fg-muted"));
        }
        plot::draw(root, &self.series, &self.labels, self.mark, count)
    }
}

impl IntoNode for ChartBuilder {
    fn build(self, cx: &mut Cx) -> sigil_kernel::NodeId {
        self.into_node_builder().build(cx)
    }
}

impl WidgetLayoutExt for ChartBuilder {
    fn layout_overrides_mut(&mut self) -> &mut LayoutOverrides {
        &mut self.layout_overrides
    }
}

/// Solid mark color for `tone`.
pub(crate) fn tone_solid(tone: Tone) -> sigil_style::Rgba8 {
    Theme::default().scale(tone.scale_role()).step(9)
}

/// Fill class for a bar of `tone`.
pub(crate) const fn tone_fill(tone: Tone) -> &'static str {
    match tone {
        Tone::Neutral => "bg-gray-9",
        Tone::Primary => "bg-accent-9",
        Tone::Info => "bg-info-9",
        Tone::Success => "bg-success-9",
        Tone::Warn => "bg-warn-9",
        Tone::Error => "bg-error-9",
    }
}

/// Round `max` up to a 1-2-5 × 10^n ceiling.
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
