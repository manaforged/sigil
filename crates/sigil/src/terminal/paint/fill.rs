use crate::geometry::Rect;
use crate::style::Theme;
use crate::widgets::spark_glyph;
use smol_str::SmolStr;

use super::{Cell, Grid, Ink, cell_edge, floor_cell};

const EIGHTHS: usize = 8;

pub(super) struct Surface<'a> {
    pub(super) rect: Rect,
    pub(super) visible: Rect,
    pub(super) origin: (usize, usize),
    pub(super) size: (usize, usize),
    pub(super) ink: Ink,
    pub(super) halves: bool,
    pub(super) theme: &'a Theme,
}

pub(super) fn surface(grid: &mut Grid, surface: &Surface<'_>, cell: f32) {
    let area = (surface.origin, surface.size, surface.rect);
    if let Some(glyph) = hairline(surface.rect, cell) {
        grid.rule(area, glyph, surface.ink);
        return;
    }
    let opaque = match surface.ink {
        Ink::Exact(color) => color.a == 255,
        Ink::Of(_) => true,
        Ink::Bare => false,
    };
    if !opaque || !surface.halves {
        grid.fill(area, surface.ink, surface.theme);
        return;
    }
    let visible = surface.visible;
    let rows = rows(
        visible.origin.y / cell,
        (visible.origin.y + visible.height) / cell,
    );
    let cols = split(
        visible.origin.x / cell,
        (visible.origin.x + visible.width) / cell,
    );
    grid.fill(
        (
            (cols.full.0, rows.full.0),
            (cols.span(), rows.span()),
            surface.rect,
        ),
        surface.ink,
        surface.theme,
    );
    halves(grid, surface, &rows, &cols);
    eighths(grid, surface, &rows, &cols);
}

fn halves(grid: &mut Grid, surface: &Surface<'_>, rows: &Rows, cols: &Split) {
    let down = (rows.full.0, rows.span());
    let halves = [
        (cols.start.map(|col| ((col, 1), down)), "▐"),
        (cols.end.map(|col| ((col, 1), down)), "▌"),
    ];
    for (span, glyph) in halves {
        if let Some(((col, width), (row, height))) = span {
            grid.rule(
                ((col, row), (width, height), surface.rect),
                glyph,
                surface.ink,
            );
        }
    }
}

fn eighths(grid: &mut Grid, surface: &Surface<'_>, rows: &Rows, cols: &Split) {
    let ink = surface.ink;
    let mut band = |row: usize, paint: &dyn Fn(&mut Cell)| {
        grid.each(
            ((cols.full.0, row), (cols.span(), 1), surface.rect),
            |cell| paint(cell),
        );
    };
    if let Some((row, level)) = rows.lower {
        band(row, &|cell| lower(cell, level, ink));
    }
    if let Some((row, level)) = rows.upper {
        band(row, &|cell| upper(cell, level, ink));
    }
}

fn lower(cell: &mut Cell, level: usize, ink: Ink) {
    *cell = Cell {
        text: block(level),
        fg: ink,
        bg: cell.bg,
        ..Cell::blank()
    };
}

fn upper(cell: &mut Cell, level: usize, ink: Ink) {
    if (1..EIGHTHS).any(|under| cell.text == block(under)) {
        cell.bg = ink;
        return;
    }
    if cell.bg.set() {
        *cell = Cell {
            text: block(EIGHTHS - level),
            fg: cell.bg,
            bg: ink,
            ..Cell::blank()
        };
        return;
    }
    let text = match level {
        0..=2 => "▔",
        3..=5 => "▀",
        _ => "█",
    };
    *cell = Cell {
        text: SmolStr::new_inline(text),
        fg: ink,
        ..Cell::blank()
    };
}

#[allow(clippy::cast_precision_loss, reason = "block levels stay below eight")]
fn block(level: usize) -> SmolStr {
    let fraction = (level as f32 - 0.5) / (EIGHTHS - 1) as f32;
    SmolStr::new(spark_glyph(fraction).encode_utf8(&mut [0; 4]))
}

struct Rows {
    full: (usize, usize),
    lower: Option<(usize, usize)>,
    upper: Option<(usize, usize)>,
}

impl Rows {
    const fn span(&self) -> usize {
        self.full.1.saturating_sub(self.full.0)
    }
}

fn rows(start: f32, end: f32) -> Rows {
    let first = eighth(start);
    let last = eighth(end).max(first + 1);
    let row = first / EIGHTHS;
    if row == (last - 1) / EIGHTHS && last - first < EIGHTHS {
        let band = Some((row, last - first));
        let floor = (row + 1) * EIGHTHS - last <= first - row * EIGHTHS;
        return Rows {
            full: (row, row),
            lower: band.filter(|_| floor),
            upper: band.filter(|_| !floor),
        };
    }
    Rows {
        full: (first.div_ceil(EIGHTHS), last / EIGHTHS),
        lower: (!first.is_multiple_of(EIGHTHS)).then_some((row, EIGHTHS - first % EIGHTHS)),
        upper: (!last.is_multiple_of(EIGHTHS)).then_some((last / EIGHTHS, last % EIGHTHS)),
    }
}

#[allow(
    clippy::cast_precision_loss,
    reason = "the eighth count is a small constant"
)]
fn eighth(value: f32) -> usize {
    floor_cell(value.mul_add(EIGHTHS as f32, 0.5))
}

struct Split {
    full: (usize, usize),
    start: Option<usize>,
    end: Option<usize>,
}

impl Split {
    const fn span(&self) -> usize {
        self.full.1.saturating_sub(self.full.0)
    }
}

fn split(start: f32, end: f32) -> Split {
    let head = start - start.floor();
    let tail = end - end.floor();
    let half_start = (0.25..0.75).contains(&head) && end - start >= 2.0;
    let half_end = (0.25..0.75).contains(&tail) && end - start >= 2.0;
    let first = if half_start {
        floor_cell(start) + 1
    } else {
        cell_edge(start)
    };
    let last = if half_end {
        floor_cell(end)
    } else {
        cell_edge(end)
    };
    let last = if half_start || half_end {
        last
    } else {
        last.max(first + 1)
    };
    Split {
        full: (first, last.max(first)),
        start: half_start.then(|| floor_cell(start)),
        end: half_end.then(|| floor_cell(end)),
    }
}

fn hairline(rect: Rect, cell: f32) -> Option<&'static str> {
    if rect.height > 0.0 && rect.height < cell * 0.25 {
        return Some("─");
    }
    if rect.width > 0.0 && rect.width < cell * 0.5 {
        return Some("│");
    }
    None
}
