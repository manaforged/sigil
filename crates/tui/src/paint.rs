use sigil::app::{Painted, cell_width};
use sigil::kernel::ImageSource;

use self::paint_clip::{image_crop, paint_text};
use crate::graphics::{Placement, blocks_window, protocol};
use sigil::style::{Rgba8, Tone};
use smol_str::SmolStr;
use unicode_segmentation::UnicodeSegmentation;

mod decoration;
#[path = "paint_clip.rs"]
mod paint_clip;

pub const CONTINUES: &str = "";

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

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Cell {
    pub text: SmolStr,
    pub fg: Ink,
    pub bg: Ink,
    pub bold: bool,
    pub italic: bool,
}

impl Cell {
    #[must_use]
    pub const fn blank() -> Self {
        Self {
            text: SmolStr::new_inline(" "),
            fg: Ink::Bare,
            bg: Ink::Bare,
            bold: false,
            italic: false,
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
    italic: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Grid {
    cols: usize,
    rows: usize,
    cells: Vec<Cell>,
    images: Vec<Placement>,
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
            images: Vec::new(),
        }
    }

    #[must_use]
    pub fn images(&self) -> &[Placement] {
        &self.images
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
        self.images.clear();
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
        self.cells.get(i).cloned()
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
            let mut line: String = slice.iter().map(|cell| cell.text.as_str()).collect();
            let trimmed = line.trim_end();
            line.truncate(trimmed.len());
            out.push_str(&line);
            if row + 1 < self.rows {
                out.push('\n');
            }
        }
        out
    }

    fn place(
        &mut self,
        source: &ImageSource,
        crop: Option<(u32, u32)>,
        at: (usize, usize),
        size: (usize, usize),
    ) {
        let (col, row) = at;
        let (width, height) = size;
        let clip = |value: usize| u16::try_from(value).unwrap_or(u16::MAX);
        self.images.push(Placement {
            col: clip(col),
            row: clip(row),
            cols: clip(width),
            rows: clip(height),
            source: source.clone(),
            crop,
        });
        if protocol().is_some() {
            return;
        }
        let window = crop.unwrap_or_else(|| (0, source.height()));
        for (dy, line) in blocks_window(source, window, width, height)
            .into_iter()
            .enumerate()
        {
            for (dx, (top, bottom)) in line.into_iter().enumerate() {
                if let Some(cell) = self
                    .index(col + dx, row + dy)
                    .and_then(|index| self.cells.get_mut(index))
                {
                    *cell = Cell {
                        text: SmolStr::new_inline("▀"),
                        fg: Ink::Exact(top),
                        bg: Ink::Exact(bottom),
                        bold: false,
                        italic: false,
                    };
                }
            }
        }
    }

    fn row_mut(&mut self, row: usize) -> &mut [Cell] {
        let start = row.saturating_mul(self.cols);
        let end = start.saturating_add(self.cols);
        self.cells.get_mut(start..end).unwrap_or_default()
    }

    fn put(&mut self, col: usize, row: usize, text: &str, fg: Ink) {
        let Some(i) = self.index(col, row) else {
            return;
        };
        if let Some(cell) = self.cells.get_mut(i) {
            if text != " " {
                cell.text = text.into();
            }
            if fg.set() {
                cell.fg = fg;
            }
        }
    }

    fn fill(
        &mut self,
        (col, row): (usize, usize),
        (width, height): (usize, usize),
        bg: Ink,
        bare: bool,
    ) {
        if !bg.set() {
            return;
        }
        let end = col.saturating_add(width);
        for y in row..row.saturating_add(height) {
            let line = self.row_mut(y);
            let span = col.min(line.len())..end.min(line.len());
            for cell in line.get_mut(span).unwrap_or_default() {
                decoration::background(cell, bg, bare);
            }
        }
    }

    fn run(&mut self, run: &Run<'_>) {
        let line = self.row_mut(run.row);
        let last = run.last.min(line.len());
        let mut x = run.col;
        for text in run.text.graphemes(true) {
            let step = cell_width(text);
            if x >= last || x.saturating_add(step) > last {
                return;
            }
            if step == 0 {
                continue;
            }
            if let Some(cell) = line.get_mut(x) {
                cell.text = text.into();
                cell.bold = run.bold;
                cell.italic = run.italic;
                if run.fg.set() {
                    cell.fg = run.fg;
                }
            }
            for pad in 1..step {
                if let Some(cell) = line.get_mut(x.saturating_add(pad)) {
                    cell.text = SmolStr::new_inline(CONTINUES);
                    cell.bold = run.bold;
                    cell.italic = run.italic;
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
        let col = cell_edge(rect.origin.x / cell);
        let row = cell_edge(rect.origin.y / cell);
        let width = cell_edge((rect.origin.x + rect.width) / cell)
            .saturating_sub(col)
            .max(1);
        let height = cell_edge((rect.origin.y + rect.height) / cell)
            .saturating_sub(row)
            .max(1);
        if let Some(source) = &node.image {
            let crop = image_crop(node, rect, source);
            grid.place(source, crop, (col, row), (width, height));
            continue;
        }
        let surface = Ink::pick(node.fill, node.tone);
        if node.text.is_none() && surface.set() {
            grid.fill((col, row), (width, height), surface, node.bare);
        }
        decoration::border(grid, node, cell);
        paint_text(grid, node, rect, cell);
    }
}

fn cell_edge(value: f32) -> usize {
    floor_cell(value + 0.5)
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
