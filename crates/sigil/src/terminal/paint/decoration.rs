use crate::geometry::Rect;
use crate::scene::CellBorder;
use crate::style::{Rgba8, Theme};
use crate::terminal::caps::Borders;
use smol_str::SmolStr;

use super::{Cell, Grid, Ink, cell_edge};

pub(super) fn background(cell: &mut Cell, ink: Ink, theme: &Theme) {
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
        text: SmolStr::new_inline(" "),
        fg: Ink::Bare,
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
    let range = |start: f32, extent: f32, limit: usize| {
        let first = cell_edge(start / cell);
        let end = cell_edge((start + extent) / cell);
        let end = if extent > 0.0 {
            end.max(first + 1)
        } else {
            end
        };
        first..end.min(limit)
    };
    (
        range(clip.origin.x, clip.width, grid.cols()),
        range(clip.origin.y, clip.height, grid.rows()),
    )
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

type Glyphs = [&'static str; 6];

const ROUNDED: Glyphs = ["╭", "╮", "╯", "╰", "─", "│"];
const HEAVY: Glyphs = ["┏", "┓", "┛", "┗", "━", "┃"];
const LIGHT: Glyphs = ["┌", "┐", "┘", "└", "─", "│"];
const ASCII: Glyphs = ["+", "+", "+", "+", "-", "|"];

fn line(border: &CellBorder, borders: Borders) -> Glyphs {
    match borders {
        Borders::Ascii => ASCII,
        Borders::Rounded if border.widths.iter().any(|width| *width >= 2.0) => HEAVY,
        Borders::Rounded if border.rounded => ROUNDED,
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
