use crate::kernel::{Cx, IntoNode, NodeId, Role, TextBuilder, container, text};
use crate::layout::prelude::{LayoutBuilderExt, LayoutStyle};
use crate::style::{NodeStyleExt, Tone};

use super::chart::tone_solid;

mod glyphs;

#[cfg(any(feature = "terminal", test))]
pub(crate) use glyphs::spark_glyph;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SparkMark {
    #[default]
    Block,
    Braille,
}

pub struct SparklineBuilder {
    values: Vec<f32>,
    range: Option<(f32, f32)>,
    tone: Tone,
    tones: Vec<Option<Tone>>,
    mark: SparkMark,
    rows: usize,
}

#[must_use]
pub fn sparkline(values: impl IntoIterator<Item = f32>) -> SparklineBuilder {
    SparklineBuilder {
        values: values.into_iter().collect(),
        range: None,
        tone: Tone::Primary,
        tones: Vec::new(),
        mark: SparkMark::Block,
        rows: 1,
    }
}

impl SparklineBuilder {
    #[must_use]
    pub const fn range(mut self, low: f32, high: f32) -> Self {
        self.range = Some((low, high));
        self
    }

    #[must_use]
    pub const fn tone(mut self, tone: Tone) -> Self {
        self.tone = tone;
        self
    }

    #[must_use]
    pub fn tones(mut self, tones: impl IntoIterator<Item = Option<Tone>>) -> Self {
        self.tones = tones.into_iter().collect();
        self
    }

    #[must_use]
    pub const fn mark(mut self, mark: SparkMark) -> Self {
        self.mark = mark;
        self
    }

    #[must_use]
    pub fn rows(mut self, rows: usize) -> Self {
        self.rows = rows.max(1);
        self
    }

    fn bounds(&self) -> (f32, f32) {
        self.range.unwrap_or_else(|| {
            let low = self.values.iter().copied().fold(f32::INFINITY, f32::min);
            let high = self
                .values
                .iter()
                .copied()
                .fold(f32::NEG_INFINITY, f32::max);
            (low.min(0.0), high)
        })
    }

    fn fractions(&self) -> Vec<f32> {
        let (low, high) = self.bounds();
        let span = high - low;
        self.values
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

    fn graph(&self) -> Vec<String> {
        glyphs::graph_rows(&self.fractions(), self.rows, self.mark)
    }

    #[cfg(test)]
    fn glyphs(&self) -> String {
        self.graph().join("\n")
    }

    fn runs(&self) -> Vec<Vec<(Tone, String)>> {
        let tones = glyphs::cell_tones(&self.tones, self.values.len(), self.mark, self.tone);
        self.graph()
            .iter()
            .map(|row| glyphs::tone_runs(row, &tones, self.tone))
            .collect()
    }
}

fn run_text(tone: Tone, glyphs: String) -> TextBuilder {
    text(glyphs).nowrap().class(tone_solid(tone).text())
}

impl IntoNode for SparklineBuilder {
    fn build(self, cx: &mut Cx) -> NodeId {
        let (low, high) = self.bounds();
        let last = self.values.last().copied().unwrap_or(0.0);
        let description = format!("{low} to {high}, last {last}");
        let mut rows = self.runs();
        if let [row] = rows.as_mut_slice()
            && let [(tone, glyphs)] = row.as_mut_slice()
        {
            return run_text(*tone, std::mem::take(glyphs))
                .role(Role::Image)
                .label("Sparkline")
                .description(description)
                .build(cx);
        }
        let mut root = container()
            .role(Role::Image)
            .label("Sparkline")
            .description(description)
            .layout(LayoutStyle::flex_column());
        for row in rows {
            let mut line = container().layout(LayoutStyle::flex_row());
            for (tone, glyphs) in row {
                line = line.child(run_text(tone, glyphs));
            }
            root = root.child(line);
        }
        root.build(cx)
    }
}

#[cfg(test)]
#[path = "sparkline_tests.rs"]
mod tests;
