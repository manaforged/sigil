use super::Grid;

const UP: u8 = 1;
const RIGHT: u8 = 2;
const DOWN: u8 = 4;
const LEFT: u8 = 8;

const LIGHT: [&str; 16] = [
    " ", "│", "─", "└", "│", "│", "┌", "├", "─", "┘", "─", "┴", "┐", "┤", "┬", "┼",
];
const HEAVY: [&str; 16] = [
    " ", "┃", "━", "┗", "┃", "┃", "┏", "┣", "━", "┛", "━", "┻", "┓", "┫", "┳", "╋",
];

const ROUNDED: [(&str, u8); 4] = [
    ("╰", UP | RIGHT),
    ("╭", RIGHT | DOWN),
    ("╮", DOWN | LEFT),
    ("╯", UP | LEFT),
];

fn arms(glyph: &str) -> Option<(u8, bool)> {
    if let Some((_, bits)) = ROUNDED.iter().find(|(corner, _)| *corner == glyph) {
        return Some((*bits, false));
    }
    let find = |table: &[&str; 16]| {
        table
            .iter()
            .rposition(|entry| *entry == glyph)
            .and_then(|index| u8::try_from(index).ok())
            .filter(|bits| *bits > 0)
    };
    find(&LIGHT)
        .map(|bits| (bits, false))
        .or_else(|| find(&HEAVY).map(|bits| (bits, true)))
}

fn at(grid: &Grid, x: usize, y: usize) -> Option<(u8, bool)> {
    if !grid.edged(x, y) {
        return None;
    }
    arms(&grid.get(x, y)?.text)
}

type Neighbour = (Option<(usize, usize)>, u8, u8);

fn neighbours(x: usize, y: usize) -> [Neighbour; 4] {
    [
        (y.checked_sub(1).map(|up| (x, up)), UP, DOWN),
        (Some((x + 1, y)), RIGHT, LEFT),
        (Some((x, y + 1)), DOWN, UP),
        (x.checked_sub(1).map(|left| (left, y)), LEFT, RIGHT),
    ]
}

fn set(grid: &mut Grid, (x, y): (usize, usize), bits: u8, heavy: bool) {
    let table = if heavy { HEAVY } else { LIGHT };
    let Some(glyph) = table.get(usize::from(bits)) else {
        return;
    };
    if let Some(cell) = grid.cell_mut(x, y) {
        cell.text = (*glyph).into();
    }
}

pub(super) fn link(grid: &mut Grid, x: usize, y: usize) {
    let Some((own, heavy)) = at(grid, x, y) else {
        return;
    };
    let mut bits = own;
    for (next, out, into) in neighbours(x, y) {
        let Some(next) = next else {
            continue;
        };
        let Some((other, other_heavy)) = at(grid, next.0, next.1) else {
            continue;
        };
        if other_heavy != heavy {
            continue;
        }
        if other & into != 0 {
            bits |= out;
        } else if own & out != 0 {
            set(grid, next, other | into, heavy);
        }
    }
    if bits != own {
        set(grid, (x, y), bits, heavy);
    }
}
