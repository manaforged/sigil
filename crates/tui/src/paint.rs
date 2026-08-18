//! Tessellate [`Painted`](sigil_app::Painted) nodes onto a character grid.

use sigil_app::Painted;
use sigil_style::Tone;

/// One terminal cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cell {
    /// Glyph. Space when this cell is only a fill.
    pub ch: char,
    /// Tone from the covering node, if any.
    pub tone: Option<Tone>,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            ch: ' ',
            tone: None,
        }
    }
}

/// A cols × rows cell buffer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Grid {
    cols: usize,
    rows: usize,
    cells: Vec<Cell>,
}

impl Grid {
    /// Empty grid of `cols` by `rows`.
    #[must_use]
    pub fn new(cols: u16, rows: u16) -> Self {
        let cols = cols as usize;
        let rows = rows as usize;
        Self {
            cols,
            rows,
            cells: vec![Cell::default(); cols.saturating_mul(rows)],
        }
    }

    /// Column count.
    #[must_use]
    pub const fn cols(&self) -> usize {
        self.cols
    }

    /// Row count.
    #[must_use]
    pub const fn rows(&self) -> usize {
        self.rows
    }

    /// Cell at (`col`, `row`), if in bounds.
    #[must_use]
    pub fn get(&self, col: usize, row: usize) -> Option<Cell> {
        let i = self.index(col, row)?;
        self.cells.get(i).copied()
    }

    /// Visible text, one line per row, trailing spaces stripped.
    #[must_use]
    pub fn text(&self) -> String {
        let mut out = String::new();
        for row in 0..self.rows {
            let start = row.saturating_mul(self.cols);
            let end = start.saturating_add(self.cols);
            let Some(slice) = self.cells.get(start..end) else {
                continue;
            };
            let mut line: String = slice.iter().map(|cell| cell.ch).collect();
            let trimmed = line.trim_end();
            line.truncate(trimmed.len());
            out.push_str(&line);
            if row + 1 < self.rows {
                out.push('\n');
            }
        }
        out
    }

    fn put(&mut self, col: usize, row: usize, ch: char, tone: Option<Tone>) {
        let Some(i) = self.index(col, row) else {
            return;
        };
        if let Some(cell) = self.cells.get_mut(i) {
            if ch != ' ' {
                cell.ch = ch;
            }
            if tone.is_some() {
                cell.tone = tone;
            }
        }
    }

    fn fill(&mut self, col: usize, row: usize, width: usize, height: usize, tone: Option<Tone>) {
        for y in row..row.saturating_add(height) {
            for x in col..col.saturating_add(width) {
                self.put(x, y, ' ', tone);
            }
        }
    }

    const fn index(&self, col: usize, row: usize) -> Option<usize> {
        if col >= self.cols || row >= self.rows {
            return None;
        }
        Some(row.saturating_mul(self.cols).saturating_add(col))
    }
}

/// Paint laid-out nodes onto a cell grid. Later nodes overwrite earlier ones.
///
/// `cell` is the px-per-cell scale used at layout (see [`sigil_app::Host::CELL`]).
#[must_use]
pub fn raster(nodes: &[Painted], cols: u16, rows: u16, cell: f32) -> Grid {
    let mut grid = Grid::new(cols, rows);
    let cell = if cell <= 0.0 { 1.0 } else { cell };
    for node in nodes {
        let col = floor_cell(node.rect.origin.x / cell);
        let row = floor_cell(node.rect.origin.y / cell);
        let width = floor_cell(node.rect.width / cell).max(1);
        let height = floor_cell(node.rect.height / cell).max(1);
        if node.tone.is_some() && node.text.is_none() {
            // Progress fill and chart bars are empty toned boxes.
            grid.fill(col, row, width, height, node.tone);
            for y in row..row.saturating_add(height) {
                for x in col..col.saturating_add(width) {
                    grid.put(x, y, '█', node.tone);
                }
            }
        } else if node.tone.is_some() {
            grid.fill(col, row, width, height, node.tone);
        }
        if let Some(text) = node.text.as_deref() {
            for (i, ch) in text.chars().enumerate() {
                grid.put(col.saturating_add(i), row, ch, node.tone);
            }
        }
    }
    grid
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn floor_cell(value: f32) -> usize {
    if value.is_sign_negative() {
        0
    } else {
        value.floor() as usize
    }
}
