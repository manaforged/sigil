//! Tessellate [`Painted`](sigil_app::Painted) nodes onto a character grid.

use sigil_app::Painted;
use sigil_style::{Rgba8, Tone};
use unicode_width::UnicodeWidthChar;

/// Second column of a wide glyph.
///
/// The glyph in the column before covers this one, so nothing prints here.
/// Printing a filler would push the rest of the line one column right.
pub const CONTINUES: char = '\0';

/// Ink for one cell: a resolved colour when the style gave one, else the
/// node's tone, else whatever the terminal already uses.
///
/// A resolved colour is the theme's own value, so a badge is the exact
/// swatch the window host would paint rather than the nearest of eight.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Ink {
    /// The terminal's own foreground or background.
    #[default]
    Bare,
    /// One of the six semantic tones.
    Of(Tone),
    /// A resolved 24-bit colour.
    Exact(Rgba8),
}

impl Ink {
    const fn pick(exact: Option<Rgba8>, tone: Option<Tone>) -> Self {
        match (exact, tone) {
            (Some(colour), _) => Self::Exact(colour),
            (None, Some(tone)) => Self::Of(tone),
            (None, None) => Self::Bare,
        }
    }

    const fn set(self) -> bool {
        !matches!(self, Self::Bare)
    }
}

/// One terminal cell.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Cell {
    /// Glyph. Space when this cell is only a fill.
    pub ch: char,
    /// Glyph colour. A text node paints this.
    pub fg: Ink,
    /// Surface colour. A box under the glyph paints this.
    pub bg: Ink,
    /// Draw the glyph bold.
    pub bold: bool,
}

impl Cell {
    /// An empty cell: a space in the terminal's own colours.
    #[must_use]
    pub const fn blank() -> Self {
        Self {
            ch: ' ',
            fg: Ink::Bare,
            bg: Ink::Bare,
            bold: false,
        }
    }
}

/// One run of glyphs bound for a single row.
///
/// `last` is the first column the run may not touch, from the clip box.
struct Run<'a> {
    col: usize,
    row: usize,
    last: usize,
    text: &'a str,
    fg: Ink,
    bold: bool,
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
            cells: vec![Cell::blank(); cols.saturating_mul(rows)],
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

    /// Blank every cell, keeping the buffer.
    pub fn reset(&mut self) {
        self.cells.fill(Cell::blank());
    }

    /// Resize to `cols` by `rows`, keeping the buffer when the area matches.
    pub fn resize(&mut self, cols: u16, rows: u16) {
        let cols = cols as usize;
        let rows = rows as usize;
        if self.cols == cols && self.rows == rows {
            self.reset();
            return;
        }
        self.cols = cols;
        self.rows = rows;
        self.cells.clear();
        self.cells
            .resize(cols.saturating_mul(rows), Cell::default());
    }

    /// Blank this grid and reshape it to match `other`.
    pub fn mirror(&mut self, other: &Self) {
        if self.cols == other.cols && self.rows == other.rows {
            self.reset();
            return;
        }
        self.cols = other.cols;
        self.rows = other.rows;
        self.cells.clear();
        self.cells
            .resize(self.cols.saturating_mul(self.rows), Cell::blank());
    }

    /// One row of cells. Empty when `row` is out of bounds.
    ///
    /// Diffing compares whole rows: one slice comparison per row instead of
    /// a bounds-checked lookup per cell.
    #[must_use]
    pub fn row(&self, row: usize) -> &[Cell] {
        let start = row.saturating_mul(self.cols);
        self.cells
            .get(start..start.saturating_add(self.cols))
            .unwrap_or_default()
    }

    /// Columns up to the last cell that paints anything on `row`.
    ///
    /// A row is mostly blank. Writing it to its last painted cell and
    /// clearing the rest costs one escape instead of a run of spaces.
    #[must_use]
    pub fn painted_width(&self, row: usize) -> usize {
        self.row(row)
            .iter()
            .rposition(|cell| *cell != Cell::blank())
            .map_or(0, |last| last.saturating_add(1))
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
            let mut line: String = slice
                .iter()
                .map(|cell| cell.ch)
                .filter(|ch| *ch != CONTINUES)
                .collect();
            let trimmed = line.trim_end();
            line.truncate(trimmed.len());
            out.push_str(&line);
            if row + 1 < self.rows {
                out.push('\n');
            }
        }
        out
    }

    /// One row as a mutable slice. Empty when `row` is out of bounds.
    ///
    /// A run of glyphs takes the slice once instead of bounds-checking the
    /// whole grid per character.
    fn row_mut(&mut self, row: usize) -> &mut [Cell] {
        let start = row.saturating_mul(self.cols);
        let end = start.saturating_add(self.cols);
        self.cells.get_mut(start..end).unwrap_or_default()
    }

    fn put(&mut self, col: usize, row: usize, ch: char, fg: Ink) {
        let Some(i) = self.index(col, row) else {
            return;
        };
        if let Some(cell) = self.cells.get_mut(i) {
            if ch != ' ' {
                cell.ch = ch;
            }
            if fg.set() {
                cell.fg = fg;
            }
        }
    }

    fn fill(&mut self, (col, row): (usize, usize), (width, height): (usize, usize), bg: Ink) {
        if !bg.set() {
            return;
        }
        let end = col.saturating_add(width);
        for y in row..row.saturating_add(height) {
            let line = self.row_mut(y);
            let span = col.min(line.len())..end.min(line.len());
            for cell in line.get_mut(span).unwrap_or_default() {
                cell.bg = bg;
            }
        }
    }

    /// Write one run of glyphs, advancing by each glyph's display width.
    fn run(&mut self, run: &Run<'_>) {
        let line = self.row_mut(run.row);
        let last = run.last.min(line.len());
        let mut x = run.col;
        for ch in run.text.chars() {
            if x >= last {
                return;
            }
            let step = UnicodeWidthChar::width(ch).unwrap_or(0);
            if let Some(cell) = line.get_mut(x) {
                cell.ch = ch;
                cell.bold = run.bold;
                if run.fg.set() {
                    cell.fg = run.fg;
                }
            }
            for pad in 1..step {
                if let Some(cell) = line.get_mut(x.saturating_add(pad)) {
                    cell.ch = CONTINUES;
                    cell.bold = run.bold;
                    if run.fg.set() {
                        cell.fg = run.fg;
                    }
                }
            }
            x = x.saturating_add(step);
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
    raster_into(&mut grid, nodes, cell);
    grid
}

/// Paint into an existing grid, reusing its cell buffer.
///
/// [`raster`] allocates a grid per call. A host painting every frame keeps
/// one and calls this, so a steady frame allocates nothing.
pub fn raster_into(grid: &mut Grid, nodes: &[Painted], cell: f32) {
    grid.reset();
    let cell = if cell <= 0.0 { 1.0 } else { cell };
    for node in nodes {
        let col = floor_cell(node.rect.origin.x / cell);
        let row = floor_cell(node.rect.origin.y / cell);
        let width = floor_cell(node.rect.width / cell).max(1);
        let height = floor_cell(node.rect.height / cell).max(1);
        // Only a box paints a surface. A text node carries its tone as ink,
        // and filling behind it would put the glyph on its own colour.
        let surface = Ink::pick(node.fill, node.tone);
        let ink = Ink::pick(node.ink, node.tone);
        if node.text.is_none() && surface.set() {
            grid.fill((col, row), (width, height), surface);
            if node.bare {
                // A progress fill and a chart bar hold nothing, so the block
                // glyph is the bar. A button surface holds its label and only
                // tints, or the label would sit in a solid slab.
                for y in row..row.saturating_add(height) {
                    for x in col..col.saturating_add(width) {
                        grid.put(x, y, '█', surface);
                    }
                }
            }
        }
        if let Some(text) = node.text.as_deref() {
            // A glyph belongs to the cell row its own middle falls in. Widget
            // chrome (a chip's padding, a button's border) offsets the text
            // inside its row; flooring the top edge would push those glyphs a
            // row down while a bare label on the same row stays put.
            // A box no taller than one cell is the row, so its text sits
            // there whatever chrome offsets the glyph inside it — a chip's
            // padding otherwise pushes the label a row below the plain
            // labels beside it. The row snaps to the nearest line because a
            // filled viewport divides its height evenly and lands rows on
            // fractional origins. Taller boxes have no row of their own and
            // fall back to the glyph's middle.
            let line = match node.clip {
                Some(clip) if clip.height <= cell => floor_cell(clip.origin.y / cell + 0.5),
                _ => floor_cell((node.rect.origin.y + node.rect.height / 2.0) / cell),
            };
            // A clipping ancestor bounds the run. Without this a wide cell
            // overprints its right-hand neighbour instead of being cut at
            // the column edge.
            let last = node.clip.map_or(usize::MAX, |clip| {
                floor_cell((clip.origin.x + clip.width) / cell)
            });
            grid.run(&Run {
                col,
                row: line,
                last,
                text,
                fg: ink,
                bold: node.weight >= 700,
            });
        }
    }
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
const fn floor_cell(value: f32) -> usize {
    if value.is_sign_negative() {
        0
    } else {
        value.floor() as usize
    }
}
