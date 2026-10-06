use crate::style::strings::StringKey;
use crate::style::theme::ambient::{ActiveTheme, theme_label};
use std::rc::Rc;

use crate::kernel::{
    AccessibleName, Cx, IntoNode, NodeBuilder, NodeId, Role, TextBuilder, container, text,
};
use crate::layout::{LayoutBuilderExt, LayoutStyle};
use crate::reactive::{Reactive, derived};
use crate::style::Tone;

mod glyphs;

#[cfg(any(feature = "terminal", test))]
pub(crate) use glyphs::spark_glyph;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SparkMark {
    #[default]
    Block,
    Braille,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SparkFill {
    #[default]
    Solid,
    Shade,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Layer {
    Edge,
    Body,
}

#[derive(Clone)]
struct SparkShape {
    range: Option<(f32, f32)>,
    tone: Tone,
    tones: Vec<Option<Tone>>,
    mark: SparkMark,
    fill: SparkFill,
    rows: usize,
}

pub struct SparklineBuilder {
    values: Rc<dyn Reactive<Vec<f32>>>,
    shape: SparkShape,
}

#[must_use]
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
    }
}

impl SparklineBuilder {
    #[must_use]
    pub const fn range(mut self, low: f32, high: f32) -> Self {
        self.shape.range = Some((low, high));
        self
    }

    #[must_use]
    pub const fn tone(mut self, tone: Tone) -> Self {
        self.shape.tone = tone;
        self
    }

    #[must_use]
    pub fn tones(mut self, tones: impl IntoIterator<Item = Option<Tone>>) -> Self {
        self.shape.tones = tones.into_iter().collect();
        self
    }

    #[must_use]
    pub const fn mark(mut self, mark: SparkMark) -> Self {
        self.shape.mark = mark;
        self
    }

    #[must_use]
    pub const fn fill(mut self, fill: SparkFill) -> Self {
        self.shape.fill = fill;
        self
    }

    #[must_use]
    pub fn rows(mut self, rows: usize) -> Self {
        self.shape.rows = rows.max(1);
        self
    }

    #[cfg(test)]
    fn graph(&self) -> Vec<String> {
        self.shape.graph(&self.values.read())
    }

    #[cfg(test)]
    fn glyphs(&self) -> String {
        self.graph().join("\n")
    }

    #[cfg(test)]
    fn runs(&self) -> Vec<Vec<(Tone, Layer, String)>> {
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

    fn runs(&self, values: &[f32]) -> Vec<Vec<(Tone, Layer, String)>> {
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

fn run_text(tone: Tone, layer: Layer, glyphs: String) -> TextBuilder {
    let ink = match layer {
        Layer::Edge => tone.solid(),
        Layer::Body => tone.shade(),
    };
    text(glyphs).nowrap().class(ink.text())
}

impl IntoNode for SparklineBuilder {
    fn build(self, cx: &mut Cx) -> NodeId {
        let source = self.values;
        let values = derived(move || source.read());
        let shape = self.shape;
        let label_shape = shape.clone();
        let theme = ActiveTheme::capture();
        let description =
            derived(move || values.with(|values| label_shape.describe(values, theme)));
        container()
            .role(Role::Image)
            .label(theme_label(StringKey::Sparkline))
            .mutate(move |target| {
                target.a11y.description = Some(AccessibleName::Derived(description));
            })
            .layout(LayoutStyle::flex_column())
            .children_reactive(move || values.with(|values| shape.lines(values)))
            .build(cx)
    }
}

#[cfg(test)]
#[path = "sparkline_tests.rs"]
mod tests;
