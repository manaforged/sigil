use crate::kernel::ImageSource;
use crate::style::Rgba8;
use crate::terminal::graphics::{Placement, sextant, sextants_window};
use smol_str::SmolStr;

use super::{Cell, Grid, Ink};

const SOLID: u32 = 51;
const INK: u8 = 64;

type Window = (usize, usize, usize);

impl Grid {
    pub(super) fn place(
        &mut self,
        source: &ImageSource,
        crop: Option<(u32, u32)>,
        at: (usize, usize),
        size: (usize, usize),
    ) {
        let clip = |value: usize| u16::try_from(value).unwrap_or(u16::MAX);
        self.images.push(Placement {
            col: clip(at.0),
            row: clip(at.1),
            cols: clip(size.0),
            rows: clip(size.1),
            source: source.clone(),
            crop,
        });
        if self.graphics {
            return;
        }
        let window = crop.unwrap_or_else(|| (0, source.height()));
        if opaque(source) {
            self.sextants(source, window, at, size);
        } else {
            self.overlay(source, window, at, size);
        }
    }

    fn sextants(
        &mut self,
        source: &ImageSource,
        window: (u32, u32),
        (col, row): (usize, usize),
        (width, height): (usize, usize),
    ) {
        for (dy, line) in sextants_window(source, window, width, height)
            .into_iter()
            .enumerate()
        {
            for (dx, (glyph, fg, bg)) in line.into_iter().enumerate() {
                if let Some(cell) = self
                    .index(col + dx, row + dy)
                    .and_then(|index| self.cells.get_mut(index))
                {
                    *cell = Cell {
                        text: SmolStr::from(glyph.to_string()),
                        fg: Ink::Exact(fg),
                        bg: Ink::Exact(bg),
                        ..Cell::blank()
                    };
                }
            }
        }
    }

    fn overlay(
        &mut self,
        source: &ImageSource,
        window: (u32, u32),
        (col, row): (usize, usize),
        (width, height): (usize, usize),
    ) {
        let area = pixels(source, window);
        if width <= 2
            && height <= 2
            && let Some((glyph, fg, (x, y))) = icon(source, pixels(source, (0, source.height())))
        {
            let dx = (x * width).min(width * 1000 - 1) / 1000;
            let dy = (y * height).min(height * 1000 - 1) / 1000;
            self.ink(col + dx, row + dy, glyph, fg);
            return;
        }
        let sums = Sums::sample(source, area, (width, height));
        for dy in 0..height {
            for dx in 0..width {
                if let Some((mask, fg)) = sums.cell(dx, dy) {
                    self.ink(col + dx, row + dy, sextant(mask), fg);
                }
            }
        }
    }

    fn ink(&mut self, col: usize, row: usize, glyph: char, fg: Rgba8) {
        if let Some(cell) = self
            .index(col, row)
            .and_then(|index| self.cells.get_mut(index))
        {
            cell.text = SmolStr::from(glyph.to_string());
            cell.fg = Ink::Exact(fg);
        }
    }
}

fn opaque(source: &ImageSource) -> bool {
    source
        .rgba()
        .as_chunks::<4>()
        .0
        .iter()
        .all(|[.., alpha]| *alpha == 255)
}

fn pixels(source: &ImageSource, window: (u32, u32)) -> Window {
    let width = source.width() as usize;
    let full = source.height() as usize;
    let top = (window.0 as usize).min(full);
    let height = (window.1 as usize).min(full.saturating_sub(top));
    (width, top, height)
}

fn texel(source: &ImageSource, x: usize, y: usize) -> [u8; 4] {
    let at = (y * source.width() as usize + x) * 4;
    source
        .rgba()
        .get(at..at + 4)
        .and_then(|px| <[u8; 4]>::try_from(px).ok())
        .unwrap_or_default()
}

struct Sums {
    cols: usize,
    slots: Vec<[u32; 5]>,
}

impl Sums {
    fn sample(source: &ImageSource, (width, top, height): Window, cells: (usize, usize)) -> Self {
        let cols = cells.0 * 2;
        let rows = cells.1 * 3;
        let mut slots = vec![[0_u32; 5]; cols * rows];
        if width == 0 || height == 0 {
            return Self { cols, slots };
        }
        for y in 0..height {
            for x in 0..width {
                let [r, g, b, a] = texel(source, x, top + y);
                let index = (y * rows / height) * cols + x * cols / width;
                if let Some(slot) = slots.get_mut(index) {
                    add(slot, [r, g, b, a]);
                }
            }
        }
        Self { cols, slots }
    }

    fn cell(&self, col: usize, row: usize) -> Option<(u32, Rgba8)> {
        let mut mask = 0;
        let mut total = [0_u32; 4];
        for bit in 0..6 {
            let index = (row * 3 + bit / 2) * self.cols + col * 2 + bit % 2;
            let [red, green, blue, alpha, weight] = *self.slots.get(index)?;
            if weight > 0 && alpha * 255 >= weight * SOLID {
                mask |= 1 << bit;
                for (sum, value) in total.iter_mut().zip([red, green, blue, alpha]) {
                    *sum += value;
                }
            }
        }
        (mask != 0).then(|| (mask, colour(total)))
    }
}

fn add([sr, sg, sb, sa, sn]: &mut [u32; 5], [r, g, b, a]: [u8; 4]) {
    let alpha = u32::from(a);
    *sr += u32::from(r) * alpha;
    *sg += u32::from(g) * alpha;
    *sb += u32::from(b) * alpha;
    *sa += alpha;
    *sn += 255;
}

fn colour([r, g, b, a]: [u32; 4]) -> Rgba8 {
    let channel = |sum: u32| u8::try_from(sum / a.max(1)).unwrap_or(u8::MAX);
    Rgba8 {
        r: channel(r),
        g: channel(g),
        b: channel(b),
        a: 255,
    }
}

type Icon = (char, Rgba8, (usize, usize));

fn icon(source: &ImageSource, (width, top, height): Window) -> Option<Icon> {
    let inked: Vec<(usize, usize, [u8; 4])> = (0..height)
        .flat_map(|y| (0..width).map(move |x| (x, y)))
        .map(|(x, y)| (x, y, texel(source, x, top + y)))
        .filter(|(.., px)| px[3] >= INK)
        .collect();
    let (x0, x1) = span(inked.iter().map(|(x, ..)| *x))?;
    let (y0, y1) = span(inked.iter().map(|(_, y, _)| *y))?;
    let bands = |along: &dyn Fn(usize, usize) -> (usize, usize), lo: usize, hi: usize| {
        let mut sums = [(0_usize, 0_usize); 3];
        for (x, y, _) in &inked {
            let (key, value) = along(*x, *y);
            let band = ((key - lo) * 3 / (hi - lo + 1)).min(2);
            if let Some(sum) = sums.get_mut(band) {
                sum.0 += value;
                sum.1 += 1;
            }
        }
        sums.map(|(sum, count)| (sum * 1000).checked_div(count))
    };
    let across = bands(&|x, y| (y, (x - x0) * 1000 / (x1 - x0 + 1)), y0, y1);
    let down = bands(&|x, y| (x, (y - y0) * 1000 / (y1 - y0 + 1)), x0, x1);
    let glyph = arrow(across, down)?;
    let mut total = [0_u32; 5];
    for (.., px) in &inked {
        add(&mut total, *px);
    }
    let [r, g, b, a, _] = total;
    let centre = (
        (x0 + x1 + 1) * 500 / width.max(1),
        (y0 + y1 + 1) * 500 / height.max(1),
    );
    Some((glyph, colour([r, g, b, a]), centre))
}

fn span(values: impl Iterator<Item = usize> + Clone) -> Option<(usize, usize)> {
    Some((values.clone().min()?, values.max()?))
}

fn arrow(across: [Option<usize>; 3], down: [Option<usize>; 3]) -> Option<char> {
    let bend = |[a, b, c]: [Option<usize>; 3]| {
        let (a, b, c) = (a? / 1000, b? / 1000, c? / 1000);
        let a = i64::try_from(a).ok()?;
        let b = i64::try_from(b).ok()?;
        let c = i64::try_from(c).ok()?;
        ((a - c).abs() < 200).then_some(b - i64::midpoint(a, c))
    };
    match (bend(across), bend(down)) {
        (Some(bow), _) if bow > 300 => Some('▸'),
        (Some(bow), _) if bow < -300 => Some('◂'),
        (_, Some(bow)) if bow > 300 => Some('▾'),
        (_, Some(bow)) if bow < -300 => Some('▴'),
        _ => None,
    }
}
