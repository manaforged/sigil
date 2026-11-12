use crate::style::strings::StringKey;
use crate::style::theme::ambient::{ActiveTheme, theme_label};
use std::rc::Rc;

use crate::kernel::{AccessibleName, NodeBuilder, Role, TextBuilder, container, text};
use crate::layout::LayoutStyle;
use crate::reactive::{Reactive, memo};
use crate::style::Tone;
use crate::widgets::Layout;

mod glyphs;

#[cfg(feature = "terminal")]
pub(crate) use glyphs::spark_glyph;

/// Sparkline glyphs.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub enum SparkMark {
    /// The block spark mark.
    #[default]
    Block,
    /// The braille spark mark.
    Braille,
}

/// Body of a multi-row sparkline.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub enum SparkFill {
    /// The solid spark fill.
    #[default]
    Solid,
    /// The shade spark fill.
    Shade,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SparkLayer {
    Edge,
    Body,
}

#[derive(Clone, Debug)]
struct SparkShape {
    range: Option<(f32, f32)>,
    tone: Tone,
    tones: Vec<Option<Tone>>,
    mark: SparkMark,
    fill: SparkFill,
    rows: usize,
}

/// Builder of a sparkline.
#[must_use]
pub struct SparklineBuilder {
    values: Rc<dyn Reactive<Vec<f32>>>,
    shape: SparkShape,
    layout_overrides: crate::widgets::layout_ext::LayoutOverrides,
}

impl core::fmt::Debug for SparklineBuilder {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SparklineBuilder")
            .field("shape", &self.shape)
            .field("layout_overrides", &self.layout_overrides)
            .finish_non_exhaustive()
    }
}

/// A sparkline.
pub fn sparkline(values: impl Reactive<Vec<f32>>) -> SparklineBuilder {
    SparklineBuilder {
        values: Rc::new(values),
        shape: SparkShape {
            range: None,
            tone: Tone::Primary,
            tones: Vec::new(),
            mark: SparkMark::Block,
            fill: SparkFill::Solid,
            rows: 1,
        },
        layout_overrides: crate::widgets::layout_ext::LayoutOverrides::new(),
    }
}

impl SparklineBuilder {
    /// Sets the range.
    pub const fn range(mut self, low: f32, high: f32) -> Self {
        self.shape.range = Some((low, high));
        self
    }

    /// Sets the tone.
    pub const fn tone(mut self, tone: Tone) -> Self {
        self.shape.tone = tone;
        self
    }

    /// Sets the tones.
    pub fn tones(mut self, tones: impl IntoIterator<Item = Option<Tone>>) -> Self {
        self.shape.tones = tones.into_iter().collect();
        self
    }

    /// Sets the mark.
    pub const fn mark(mut self, mark: SparkMark) -> Self {
        self.shape.mark = mark;
        self
    }

    /// Sets how the area under the line is drawn.
    pub const fn area(mut self, fill: SparkFill) -> Self {
        self.shape.fill = fill;
        self
    }

    /// Sets the rows.
    pub fn rows(mut self, rows: usize) -> Self {
        self.shape.rows = rows.max(1);
        self
    }

    #[cfg(test)]
    fn graph(&self) -> Vec<String> {
        self.shape.graph(&self.values.read())
    }

    #[cfg(test)]
    fn runs(&self) -> Vec<Vec<(Tone, SparkLayer, String)>> {
        self.shape.runs(&self.values.read())
    }
}

impl SparkShape {
    fn bounds(&self, values: &[f32]) -> (f32, f32) {
        self.range.unwrap_or_else(|| {
            let low = values.iter().copied().fold(f32::INFINITY, f32::min);
            let high = values.iter().copied().fold(f32::NEG_INFINITY, f32::max);
            (low.min(0.0), high)
        })
    }

    fn fractions(&self, values: &[f32]) -> Vec<f32> {
        let (low, high) = self.bounds(values);
        let span = high - low;
        values
            .iter()
            .map(|value| {
                if span > 0.0 {
                    (value - low) / span
                } else {
                    0.0
                }
            })
            .collect()
    }

    fn graph(&self, values: &[f32]) -> Vec<String> {
        glyphs::graph_rows(&self.fractions(values), self.rows, self.mark)
    }

    fn runs(&self, values: &[f32]) -> Vec<Vec<(Tone, SparkLayer, String)>> {
        let tones = glyphs::cell_tones(&self.tones, values.len(), self.mark, self.tone);
        let graph = self.graph(values);
        let layers = glyphs::cell_layers(&graph, self.fill);
        graph
            .iter()
            .zip(&layers)
            .map(|(row, layer)| glyphs::tone_runs(row, &tones, layer, self.tone))
            .collect()
    }

    fn describe(&self, values: &[f32], theme: ActiveTheme) -> String {
        let (low, high) = self.bounds(values);
        let last = values.last().copied().unwrap_or(0.0);
        theme.fill(StringKey::SparklineSummary, &[&low, &high, &last])
    }

    fn lines(&self, values: &[f32]) -> Vec<NodeBuilder> {
        self.runs(values)
            .into_iter()
            .map(|row| {
                row.into_iter().fold(
                    container().layout(LayoutStyle::flex_row()),
                    |line, (tone, layer, glyphs)| line.child(run_text(tone, layer, glyphs)),
                )
            })
            .collect()
    }
}

fn run_text(tone: Tone, layer: SparkLayer, glyphs: String) -> TextBuilder {
    let ink = match layer {
        SparkLayer::Edge => tone.solid(),
        SparkLayer::Body => tone.shade(),
    };
    text(glyphs).nowrap().class(ink.text())
}

impl SparklineBuilder {
    fn root(self) -> NodeBuilder {
        let source = self.values;
        let values = memo(move || source.read());
        let shape = self.shape;
        let label_shape = shape.clone();
        let theme = ActiveTheme::capture();
        let description = memo(move || values.with(|values| label_shape.describe(values, theme)));
        container()
            .role(Role::Image)
            .label(theme_label(StringKey::Sparkline))
            .mutate(move |target| {
                target.a11y.description = Some(AccessibleName::Derived(description));
            })
            .layout(LayoutStyle::flex_column())
            .children_reactive(move || values.with(|values| shape.lines(values)))
    }
}

builder_impls!([] SparklineBuilder: widget, node);

#[cfg(test)]
#[path = "sparkline_tests.rs"]
mod tests;
