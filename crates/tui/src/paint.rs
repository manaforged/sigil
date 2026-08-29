use sigil_app::Painted;
use sigil_style::{Rgba8, Tone};
use unicode_width::UnicodeWidthChar;

pub const CONTINUES: char = '\0';

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Ink {
    #[default]
    Bare,
    Of(Tone),
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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Cell {
    pub ch: char,
    pub fg: Ink,
    pub bg: Ink,
    pub bold: bool,
}

impl Cell {
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

struct Run<'a> {
    col: usize,
    row: usize,
    last: usize,
    text: &'a str,
    fg: Ink,
    bold: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Grid {
    cols: usize,
    rows: usize,
    cells: Vec<Cell>,
}

impl Grid {
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

    #[must_use]
    pub const fn cols(&self) -> usize {
        self.cols
    }

    #[must_use]
    pub const fn rows(&self) -> usize {
        self.rows
    }

    pub fn reset(&mut self) {
        self.cells.fill(Cell::blank());
    }

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

    #[must_use]
    pub fn row(&self, row: usize) -> &[Cell] {
        let start = row.saturating_mul(self.cols);
        self.cells
            .get(start..start.saturating_add(self.cols))
            .unwrap_or_default()
    }

    #[must_use]
    pub fn painted_width(&self, row: usize) -> usize {
        self.row(row)
            .iter()
            .rposition(|cell| *cell != Cell::blank())
            .map_or(0, |last| last.saturating_add(1))
    }

    #[must_use]
    pub fn get(&self, col: usize, row: usize) -> Option<Cell> {
        let i = self.index(col, row)?;
        self.cells.get(i).copied()
    }

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

#[must_use]
pub fn raster(nodes: &[Painted], cols: u16, rows: u16, cell: f32) -> Grid {
    let mut grid = Grid::new(cols, rows);
    raster_into(&mut grid, nodes, cell);
    grid
}

pub fn raster_into(grid: &mut Grid, nodes: &[Painted], cell: f32) {
    grid.reset();
    let cell = if cell <= 0.0 { 1.0 } else { cell };
    for node in nodes {
        let mut rect = node.rect;
        if let Some(clip) = node.clip {
            let Some(c) = rect.intersection(clip) else {
                continue;
            };
            rect = c;
        }
        let col = floor_cell(rect.origin.x / cell);
        let row = floor_cell(rect.origin.y / cell);
        let width = floor_cell(rect.width / cell).max(1);
        let height = floor_cell(rect.height / cell).max(1);
        let surface = Ink::pick(node.fill, node.tone);
        let ink = Ink::pick(node.ink, node.tone);
        if node.text.is_none() && surface.set() {
            grid.fill((col, row), (width, height), surface);
            if node.bare {
                for y in row..row.saturating_add(height) {
                    for x in col..col.saturating_add(width) {
                        grid.put(x, y, '█', surface);
                    }
                }
            }
        }
        if let Some(text) = node.text.as_deref() {
            let line = match node.clip {
                Some(clip) if clip.height <= cell => floor_cell(clip.origin.y / cell + 0.5),
                _ => floor_cell((node.rect.origin.y + node.rect.height / 2.0) / cell),
            };
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

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "values are bounded to the target range before the cast"
)]
const fn floor_cell(value: f32) -> usize {
    if value.is_sign_negative() {
        0
    } else {
        value.floor() as usize
    }
}
