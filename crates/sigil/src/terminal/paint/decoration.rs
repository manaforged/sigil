use crate::geometry::Rect;
use crate::scene::CellBorder;
use crate::style::{Rgba8, Theme};
use crate::terminal::caps::Borders;
use smol_str::SmolStr;

use super::{Cell, Grid, Ink, cell_edge, floor_cell};

pub(super) fn background(cell: &mut Cell, ink: Ink, bare: bool, theme: &Theme) {
    if let Ink::Exact(over) = ink {
        if over.a == 0 {
            return;
        }
        if over.a < 255 {
            cell.bg = blend(cell.bg, over, false, theme);
            cell.fg = blend(cell.fg, over, true, theme);
            return;
        }
    }
    *cell = Cell {
        text: SmolStr::new_inline(if bare { "█" } else { " " }),
        fg: if bare { ink } else { Ink::Bare },
        bg: ink,
        ..Cell::blank()
    };
}

pub(super) fn over(under: Ink, ink: Ink, theme: &Theme) -> Ink {
    match ink {
        Ink::Exact(color) if color.a < 255 => blend(under, color, false, theme),
        Ink::Bare | Ink::Of(_) | Ink::Exact(_) => ink,
    }
}

fn blend(ink: Ink, over: Rgba8, foreground: bool, theme: &Theme) -> Ink {
    let under = match ink {
        Ink::Exact(color) => color,
        Ink::Of(tone) => theme.scale(tone.scale_role()).step(11),
        Ink::Bare if foreground => theme.on_surface,
        Ink::Bare => theme.surface,
    };
    let alpha = u16::from(over.a);
    let channel = |top: u8, bottom: u8| {
        u8::try_from((u16::from(top) * alpha + u16::from(bottom) * (255 - alpha) + 127) / 255)
            .unwrap_or(u8::MAX)
    };
    Ink::Exact(Rgba8 {
        r: channel(over.r, under.r),
        g: channel(over.g, under.g),
        b: channel(over.b, under.b),
        a: 255,
    })
}

pub(super) struct Surface<'a> {
    pub(super) rect: Rect,
    pub(super) visible: Rect,
    pub(super) origin: (usize, usize),
    pub(super) size: (usize, usize),
    pub(super) ink: Ink,
    pub(super) leaf: bool,
    pub(super) halves: bool,
    pub(super) theme: &'a Theme,
}

pub(super) fn surface(grid: &mut Grid, surface: &Surface<'_>, cell: f32) {
    if let Some(glyph) = hairline(surface.rect, cell) {
        grid.rule(
            (surface.origin, surface.size, surface.rect),
            glyph,
            surface.ink,
        );
        return;
    }
    let opaque = match surface.ink {
        Ink::Exact(color) => color.a == 255,
        Ink::Of(_) => true,
        Ink::Bare => false,
    };
    if !opaque || !surface.halves {
        grid.fill(
            (surface.origin, surface.size, surface.rect),
            surface.ink,
            (surface.leaf, surface.theme),
        );
        return;
    }
    let visible = surface.visible;
    let rows = split(
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
        (surface.leaf, surface.theme),
    );
    halves(grid, surface, &rows, &cols);
}

fn halves(grid: &mut Grid, surface: &Surface<'_>, rows: &Split, cols: &Split) {
    let across = (cols.full.0, cols.span());
    let down = (rows.full.0, rows.span());
    let halves = [
        (rows.start.map(|row| (across, (row, 1))), "▄"),
        (rows.end.map(|row| (across, (row, 1))), "▀"),
        (cols.start.map(|col| ((col, 1), down)), "▐"),
        (cols.end.map(|col| ((col, 1), down)), "▌"),
    ];
    for (span, glyph) in halves {
        if let Some(((col, width), (row, height))) = span {
            grid.half(
                ((col, row), (width, height), surface.rect),
                glyph,
                surface.ink,
            );
        }
    }
}

impl Grid {
    fn half(&mut self, area: super::Area, glyph: &str, ink: Ink) {
        self.each(area, |cell| {
            cell.text = glyph.into();
            cell.fg = ink;
        });
    }
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
    if rect.height > 0.0 && rect.height < cell {
        return Some("─");
    }
    if rect.width > 0.0 && rect.width < cell {
        return Some("│");
    }
    None
}

pub(super) struct Edge<'a> {
    pub(super) clip: Option<Rect>,
    pub(super) theme: &'a Theme,
    pub(super) borders: Borders,
    pub(super) cell: f32,
}

pub(super) fn border(grid: &mut Grid, border: &CellBorder, edge: &Edge<'_>) {
    let Edge {
        clip, theme, cell, ..
    } = *edge;
    let color = border.color;
    if color.a == 0 || border.widths.iter().all(|width| *width <= 0.0) {
        return;
    }
    let rect = border.rect;
    let Some(clip) = clip.map_or(Some(rect), |clip| rect.intersection(clip)) else {
        return;
    };
    let left = cell_edge(rect.origin.x / cell);
    let top = cell_edge(rect.origin.y / cell);
    let right = cell_edge((rect.origin.x + rect.width) / cell).saturating_sub(1);
    let bottom = cell_edge((rect.origin.y + rect.height) / cell).saturating_sub(1);
    let ([north, east, south, west], collapsed) = sides(border, top, bottom);
    let set = line(border, edge.borders);
    let (xs, ys) = clip_cells(grid, clip, cell);
    for y in ys {
        for x in xs.clone() {
            let edges = [
                north && y == top && rect.origin.y >= 0.0,
                east && x == right,
                south && y == bottom,
                west && x == left && rect.origin.x >= 0.0,
            ];
            if let Some(glyph) = glyph(edges, set) {
                let stroke = Stroke {
                    at: (x, y),
                    glyph,
                    set,
                    side: (x == left, x == right),
                    full: !collapsed,
                };
                join(grid, &stroke, (color, theme), rect);
            }
        }
    }
}

fn clip_cells(
    grid: &Grid,
    clip: Rect,
    cell: f32,
) -> (std::ops::Range<usize>, std::ops::Range<usize>) {
    let start_x = cell_edge(clip.origin.x / cell);
    let start_y = cell_edge(clip.origin.y / cell);
    let end_x = span(
        start_x,
        (clip.origin.x + clip.width) / cell,
        clip.width,
        grid.cols(),
    );
    let end_y = span(
        start_y,
        (clip.origin.y + clip.height) / cell,
        clip.height,
        grid.rows(),
    );
    (start_x..end_x, start_y..end_y)
}

fn sides(border: &CellBorder, top: usize, bottom: usize) -> ([bool; 4], bool) {
    let [north, east, south, west] = border.widths.map(|width| width > 0.0);
    let rule = if east || west {
        bottom < top + 2
    } else {
        top == bottom
    };
    (
        [north && !rule, east, south && !rule, west],
        rule && (north || south),
    )
}

struct Stroke {
    at: (usize, usize),
    glyph: &'static str,
    set: Glyphs,
    side: (bool, bool),
    full: bool,
}

fn join(grid: &mut Grid, stroke: &Stroke, (color, theme): (Rgba8, &Theme), rect: Rect) {
    let (x, y) = stroke.at;
    let [.., across, down] = stroke.set;
    let under = grid.get(x, y);
    let bg = under.as_ref().map_or(Ink::Bare, |cell| cell.bg);
    if !stroke.full {
        grid.put(x, y, stroke.glyph, blend(bg, color, false, theme));
        return;
    }
    let crossing =
        stroke.glyph == across && grid.edged(x, y) && under.is_some_and(|cell| cell.text == down);
    let inner =
        |col: usize| grid.edged(col, y) && grid.get(col, y).is_some_and(|cell| cell.text == across);
    let ruled = stroke.glyph == down
        && ((stroke.side.0 && inner(x + 1)) || (stroke.side.1 && x > 0 && inner(x - 1)));
    let glyph = match (crossing || ruled, stroke.side) {
        (true, (true, _)) => tee(stroke.set).0,
        (true, (_, true)) => tee(stroke.set).1,
        _ => stroke.glyph,
    };
    grid.put(x, y, glyph, blend(bg, color, false, theme));
    if !crossing {
        grid.edge(x, y, rect);
    }
    super::junction::link(grid, x, y);
    if stroke.glyph == across {
        let (west, east) = tee(stroke.set);
        if stroke.side.0 && x > 0 {
            meet(grid, (x - 1, y), down, west);
        }
        if stroke.side.1 {
            meet(grid, (x + 1, y), down, east);
        }
    }
}

fn meet(grid: &mut Grid, (x, y): (usize, usize), down: &str, glyph: &str) {
    let Some(cell) = grid.get(x, y) else {
        return;
    };
    if grid.edged(x, y) && cell.text == down {
        grid.put(x, y, glyph, cell.fg);
    }
}

fn tee(set: Glyphs) -> (&'static str, &'static str) {
    if set == HEAVY {
        ("┣", "┫")
    } else if set == ASCII {
        ("+", "+")
    } else {
        ("├", "┤")
    }
}

impl Grid {
    pub(super) fn edged(&self, col: usize, row: usize) -> bool {
        self.index(col, row)
            .and_then(|index| self.edges.get(index))
            .is_some_and(Option::is_some)
    }
}

fn span(start: usize, edge: f32, extent: f32, limit: usize) -> usize {
    let end = cell_edge(edge);
    let end = if extent > 0.0 {
        end.max(start + 1)
    } else {
        end
    };
    end.min(limit)
}

type Glyphs = [&'static str; 6];

const ROUNDED: Glyphs = ["╭", "╮", "╯", "╰", "─", "│"];
const HEAVY: Glyphs = ["┏", "┓", "┛", "┗", "━", "┃"];
const LIGHT: Glyphs = ["┌", "┐", "┘", "└", "─", "│"];
const ASCII: Glyphs = ["+", "+", "+", "+", "-", "|"];

fn line(border: &CellBorder, borders: Borders) -> Glyphs {
    match borders {
        Borders::Ascii => ASCII,
        Borders::Rounded if border.rounded => ROUNDED,
        Borders::Rounded if border.widths.iter().any(|width| *width >= 2.0) => HEAVY,
        Borders::Light | Borders::Rounded => LIGHT,
    }
}

const fn glyph(edges: [bool; 4], set: Glyphs) -> Option<&'static str> {
    let [north_west, north_east, south_east, south_west, across, down] = set;
    match edges {
        [true, _, _, true] => Some(north_west),
        [true, true, _, _] => Some(north_east),
        [_, true, true, _] => Some(south_east),
        [_, _, true, true] => Some(south_west),
        [true, _, _, _] | [_, _, true, _] => Some(across),
        [_, true, _, _] | [_, _, _, true] => Some(down),
        _ => None,
    }
}
