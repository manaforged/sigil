use crate::style::Tone;

use super::{Layer, SparkFill, SparkMark};

pub(super) const SPARKS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
const BLOCK_LEVELS: usize = 8;
const BRAILLE_LEVELS: usize = 4;
const BRAILLE_BASE: u32 = 0x2800;
const BRAILLE_LEFT: [u32; 4] = [0x01, 0x02, 0x04, 0x40];
const BRAILLE_RIGHT: [u32; 4] = [0x08, 0x10, 0x20, 0x80];
const EMPTY: char = ' ';

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the level is clamped to 0..=7 before the cast"
)]
pub(crate) fn spark_glyph(fraction: f32) -> char {
    let level = (fraction.clamp(0.0, 1.0) * 7.0).floor() as usize;
    SPARKS.get(level).copied().unwrap_or('▁')
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "the fraction is clamped to 0..=1, so the level fits in usize"
)]
fn levels(fractions: &[f32], levels: usize) -> Vec<usize> {
    fractions
        .iter()
        .map(|fraction| (fraction.clamp(0.0, 1.0) * levels as f32).round() as usize)
        .collect()
}

fn cell_fill(level: usize, row_from_bottom: usize, per_row: usize) -> usize {
    level.saturating_sub(row_from_bottom * per_row).min(per_row)
}

pub(super) fn graph_rows(fractions: &[f32], rows: usize, mark: SparkMark) -> Vec<String> {
    let rows = rows.max(1);
    match mark {
        SparkMark::Block if rows == 1 => vec![fractions.iter().copied().map(spark_glyph).collect()],
        SparkMark::Block => block_rows(fractions, rows),
        SparkMark::Braille => braille_rows(fractions, rows),
    }
}

fn block_rows(fractions: &[f32], rows: usize) -> Vec<String> {
    let levels = levels(fractions, BLOCK_LEVELS * rows);
    (0..rows)
        .rev()
        .map(|row| {
            levels
                .iter()
                .map(|level| block_cell(cell_fill(*level, row, BLOCK_LEVELS)))
                .collect()
        })
        .collect()
}

fn block_cell(fill: usize) -> char {
    fill.checked_sub(1)
        .and_then(|index| SPARKS.get(index).copied())
        .unwrap_or(EMPTY)
}

fn braille_rows(fractions: &[f32], rows: usize) -> Vec<String> {
    let levels = levels(fractions, BRAILLE_LEVELS * rows);
    let fill = |pair: &[usize], side: usize, row| {
        pair.get(side)
            .map_or(0, |level| cell_fill(*level, row, BRAILLE_LEVELS))
    };
    (0..rows)
        .rev()
        .map(|row| {
            levels
                .chunks(2)
                .map(|pair| braille_cell(fill(pair, 0, row), fill(pair, 1, row)))
                .collect()
        })
        .collect()
}

fn braille_cell(left: usize, right: usize) -> char {
    let bits = column_bits(&BRAILLE_LEFT, left) | column_bits(&BRAILLE_RIGHT, right);
    char::from_u32(BRAILLE_BASE | bits).unwrap_or(EMPTY)
}

fn column_bits(column: &[u32; 4], fill: usize) -> u32 {
    column
        .iter()
        .rev()
        .take(fill)
        .fold(0, |bits, bit| bits | bit)
}

pub(super) fn cell_tones(
    tones: &[Option<Tone>],
    values: usize,
    mark: SparkMark,
    base: Tone,
) -> Vec<Tone> {
    let tone_at = |index: usize| tones.get(index).copied().flatten();
    match mark {
        SparkMark::Block => (0..values).map(|i| tone_at(i).unwrap_or(base)).collect(),
        SparkMark::Braille => (0..values.div_ceil(2))
            .map(|cell| {
                tone_at(cell * 2 + 1)
                    .or_else(|| tone_at(cell * 2))
                    .unwrap_or(base)
            })
            .collect(),
    }
}

pub(super) fn cell_layers(graph: &[String], fill: SparkFill) -> Vec<Vec<Layer>> {
    let rows: Vec<Vec<char>> = graph.iter().map(|row| row.chars().collect()).collect();
    let width = rows.iter().map(Vec::len).max().unwrap_or(0);
    let edge: Vec<Option<usize>> = (0..width)
        .map(|col| {
            rows.iter()
                .position(|row| row.get(col).is_some_and(|glyph| *glyph != EMPTY))
        })
        .collect();
    rows.iter()
        .enumerate()
        .map(|(index, row)| {
            (0..row.len())
                .map(|col| match (fill, edge.get(col).copied().flatten()) {
                    (SparkFill::Shade, Some(top)) if index > top => Layer::Body,
                    _ => Layer::Edge,
                })
                .collect()
        })
        .collect()
}

pub(super) fn tone_runs(
    row: &str,
    tones: &[Tone],
    layers: &[Layer],
    base: Tone,
) -> Vec<(Tone, Layer, String)> {
    let mut runs: Vec<(Tone, Layer, String)> = Vec::new();
    for (index, glyph) in row.chars().enumerate() {
        let tone = tones.get(index).copied().unwrap_or(base);
        let layer = layers.get(index).copied().unwrap_or(Layer::Edge);
        match runs.last_mut() {
            Some((last, kind, text)) if *last == tone && *kind == layer => text.push(glyph),
            _ => runs.push((tone, layer, glyph.to_string())),
        }
    }
    runs
}
