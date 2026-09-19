use sigil::app::Painted;
use sigil::style::{Rgba8, Theme};
use smol_str::SmolStr;

use super::{Cell, Grid, Ink, cell_edge};

pub(super) fn background(cell: &mut Cell, ink: Ink, bare: bool) {
    if let Ink::Exact(over) = ink {
        if over.a == 0 {
            return;
        }
        if over.a < 255 {
            cell.bg = blend(cell.bg, over, false);
            cell.fg = blend(cell.fg, over, true);
            return;
        }
    }
    *cell = Cell {
        text: SmolStr::new_inline(if bare { "█" } else { " " }),
        fg: if bare { ink } else { Ink::Bare },
        bg: ink,
        bold: false,
        italic: false,
    };
}

fn blend(ink: Ink, over: Rgba8, foreground: bool) -> Ink {
    let under = match ink {
        Ink::Exact(color) => color,
        Ink::Of(tone) => Theme::default().scale(tone.scale_role()).step(11),
        Ink::Bare if foreground => Theme::default().on_surface,
        Ink::Bare => Theme::default().surface,
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

pub(super) fn border(grid: &mut Grid, node: &Painted, cell: f32) {
    let Some(color) = node.border_color.filter(|color| color.a > 0) else {
        return;
    };
    if node.border_widths.iter().all(|width| *width <= 0.0) {
        return;
    }
    let rect = node.rect;
    let Some(clip) = node.clip.map_or(Some(rect), |clip| rect.intersection(clip)) else {
        return;
    };
    let left = cell_edge(rect.origin.x / cell);
    let top = cell_edge(rect.origin.y / cell);
    let right = cell_edge((rect.origin.x + rect.width) / cell).saturating_sub(1);
    let bottom = cell_edge((rect.origin.y + rect.height) / cell).saturating_sub(1);
    let [north, east, south, west] = node.border_widths.map(|width| width > 0.0);
    let end_x = cell_edge((clip.origin.x + clip.width) / cell).min(grid.cols());
    let end_y = cell_edge((clip.origin.y + clip.height) / cell).min(grid.rows());
    for y in cell_edge(clip.origin.y / cell)..end_y {
        for x in cell_edge(clip.origin.x / cell)..end_x {
            let edges = [
                north && y == top && rect.origin.y >= 0.0,
                east && x == right,
                south && y == bottom,
                west && x == left && rect.origin.x >= 0.0,
            ];
            if let Some(glyph) = glyph(edges, node.border_radius > 0.0) {
                let under = grid.get(x, y).map_or(Ink::Bare, |under| under.bg);
                grid.put(x, y, glyph, blend(under, color, false));
            }
        }
    }
}

const fn glyph(edges: [bool; 4], rounded: bool) -> Option<&'static str> {
    match edges {
        [true, _, _, true] => Some(if rounded { "╭" } else { "┌" }),
        [true, true, _, _] => Some(if rounded { "╮" } else { "┐" }),
        [_, true, true, _] => Some(if rounded { "╯" } else { "┘" }),
        [_, _, true, true] => Some(if rounded { "╰" } else { "└" }),
        [true, _, _, _] | [_, _, true, _] => Some("─"),
        [_, true, _, _] | [_, _, _, true] => Some("│"),
        _ => None,
    }
}
