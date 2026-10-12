use crate::geometry::Rect;

use crate::style::{Rgba8, Theme, Tone};
use crate::terminal::glyphs::Glyphs;
use crate::terminal::graphics::Placement;
use smol_str::SmolStr;
use std::borrow::Cow;
use unicode_segmentation::UnicodeSegmentation;

mod decoration;
mod image;
mod junction;
mod marks;
#[path = "paint_clip.rs"]
mod paint_clip;
mod scene;
mod transform;

pub use marks::Marks;
pub use scene::{Paint, raster, raster_into, raster_with};

pub const CONTINUES: &str = "";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum Ink {
    #[default]
    Bare,
    Of(Tone),
    Exact(Rgba8),
}

impl Ink {
    const fn pick(colour: Rgba8, tone: Option<Tone>) -> Self {
        match (colour, tone) {
            (colour, _) if colour.a > 0 => Self::Exact(colour),
            (_, Some(tone)) => Self::Of(tone),
            (_, None) => Self::Bare,
        }
    }

    pub(crate) const fn set(self) -> bool {
        !matches!(self, Self::Bare)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Underline {
    #[default]
    None,
    Straight,
    Curly,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Lines {
    pub underline: Underline,
    pub strike: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Cell {
    pub text: SmolStr,
    pub fg: Ink,
    pub bg: Ink,
    pub bold: bool,
    pub italic: bool,
    pub dim: bool,
    pub lines: Lines,
    pub underline_ink: Ink,
    pub link: Option<SmolStr>,
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
            dim: false,
            lines: Lines {
                underline: Underline::None,
                strike: false,
            },
            underline_ink: Ink::Bare,
            link: None,
        }
    }
}

#[derive(Clone, Copy, Default)]
pub(super) struct Face {
    pub(super) bold: bool,
    pub(super) italic: bool,
    pub(super) dim: bool,
    pub(super) underline: Underline,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Grid {
    cols: usize,
    rows: usize,
    cells: Vec<Cell>,
    edges: Vec<Option<Frame>>,
    images: Vec<Placement>,
    graphics: bool,
    cursor: Option<(usize, usize)>,
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
            edges: vec![None; cols.saturating_mul(rows)],
            images: Vec::new(),
            graphics: false,
            cursor: None,
        }
    }

    pub const fn set_graphics(&mut self, graphics: bool) {
        self.graphics = graphics;
    }

    #[must_use]
    pub const fn cursor(&self) -> Option<(usize, usize)> {
        self.cursor
    }

    pub(crate) const fn place_cursor(&mut self, at: Option<(usize, usize)>) {
        self.cursor = at;
    }

    pub(crate) fn set_row(&mut self, row: usize, cells: &[Cell]) {
        let start = row.saturating_mul(self.cols);
        if let Some(slot) = self.cells.get_mut(start..start.saturating_add(self.cols)) {
            for (cell, next) in slot.iter_mut().zip(cells) {
                cell.clone_from(next);
            }
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
        self.edges.fill(None);
        self.images.clear();
        self.cursor = None;
    }

    pub(super) fn map_glyphs(&mut self, glyphs: Glyphs) {
        if glyphs == Glyphs::Unicode {
            return;
        }
        for cell in &mut self.cells {
            let Cow::Owned(text) = glyphs.text(&cell.text) else {
                continue;
            };
            cell.text = text.into();
        }
    }

    pub fn resize(&mut self, cols: u16, rows: u16) {
        self.reshape(cols as usize, rows as usize, Cell::default());
        self.cursor = None;
    }

    pub fn mirror(&mut self, other: &Self) {
        self.reshape(other.cols, other.rows, Cell::blank());
    }

    fn reshape(&mut self, cols: usize, rows: usize, fill: Cell) {
        if (self.cols, self.rows) == (cols, rows) {
            self.reset();
            return;
        }
        self.cols = cols;
        self.rows = rows;
        self.cells.clear();
        self.cells.resize(cols.saturating_mul(rows), fill);
        self.edges.clear();
        self.edges.resize(cols.saturating_mul(rows), None);
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
            let line: String = self
                .row(row)
                .iter()
                .map(|cell| cell.text.as_str())
                .collect();
            out.push_str(line.trim_end());
            if row + 1 < self.rows {
                out.push('\n');
            }
        }
        out
    }

    pub(crate) fn overlay_text(&mut self, col: usize, row: usize, text: &str, fg: Ink) {
        let mut x = col;
        for grapheme in text.graphemes(true) {
            let step = crate::app::cell_width(grapheme);
            if step == 0 {
                continue;
            }
            if x.saturating_add(step) > self.cols {
                return;
            }
            for pad in 0..step {
                let Some(cell) = self.cell_mut(x + pad, row) else {
                    return;
                };
                cell.text = cell_text(grapheme, pad);
                cell.fg = fg;
            }
            x += step;
        }
    }

    fn put(&mut self, col: usize, row: usize, text: &str, fg: Ink) {
        if let Some(cell) = self.cell_mut(col, row) {
            if text != " " {
                cell.text = text.into();
            }
            if fg.set() {
                cell.fg = fg;
            }
        }
    }

    fn fill(&mut self, area: Area, bg: Ink, theme: &Theme) {
        if !bg.set() {
            return;
        }
        self.each(area, |cell| decoration::background(cell, bg, theme));
    }

    fn rule(&mut self, area: Area, glyph: &str, ink: Ink) {
        self.each(area, |cell| {
            cell.text = glyph.into();
            if ink.set() {
                cell.fg = ink;
            }
        });
    }

    fn each(&mut self, area: Area, mut paint: impl FnMut(&mut Cell)) {
        let ((col, row), (width, height), inside) = area;
        let inner = frame(inside);
        for y in row..row.saturating_add(height).min(self.rows) {
            for x in col..col.saturating_add(width).min(self.cols) {
                let Some(index) = self.index(x, y) else {
                    continue;
                };
                let edge = self.edges.get(index).copied().flatten();
                if edge.is_some_and(|edge| encloses(edge, inner)) {
                    continue;
                }
                if let Some(slot) = self.edges.get_mut(index) {
                    *slot = None;
                }
                if let Some(cell) = self.cells.get_mut(index) {
                    paint(cell);
                }
            }
        }
    }

    fn edge(&mut self, col: usize, row: usize, owner: Rect) {
        if let Some(slot) = self
            .index(col, row)
            .and_then(|index| self.edges.get_mut(index))
        {
            *slot = Some(frame(owner));
        }
    }

    pub(super) fn cell_mut(&mut self, col: usize, row: usize) -> Option<&mut Cell> {
        self.index(col, row)
            .and_then(|index| self.cells.get_mut(index))
    }

    pub(super) const fn index(&self, col: usize, row: usize) -> Option<usize> {
        if col >= self.cols || row >= self.rows {
            return None;
        }
        Some(row.saturating_mul(self.cols).saturating_add(col))
    }
}

fn cell_text(grapheme: &str, pad: usize) -> SmolStr {
    if pad == 0 {
        crate::app::cell_glyph(grapheme).into()
    } else {
        SmolStr::new_inline(CONTINUES)
    }
}

type Area = ((usize, usize), (usize, usize), Rect);

type Frame = [i32; 4];

#[allow(
    clippy::cast_possible_truncation,
    reason = "scene coordinates fit the i32 range after rounding"
)]
fn frame(rect: Rect) -> Frame {
    let px = |value: f32| value.round() as i32;
    [
        px(rect.origin.x),
        px(rect.origin.y),
        px(rect.origin.x + rect.width),
        px(rect.origin.y + rect.height),
    ]
}

fn encloses(outer: Frame, inner: Frame) -> bool {
    let [left, top, right, bottom] = outer;
    let [x0, y0, x1, y1] = inner;
    x0 >= left && y0 >= top && x1 <= right + 1 && y1 <= bottom + 1 && inner != outer
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
