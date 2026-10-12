use std::collections::BTreeSet;
use std::fmt::Write as _;

use super::encode::{Sample, premultiplied};
use crate::kernel::ImageSource;

const CELL_W: usize = 8;
const CELL_H: usize = 16;
const BAND: usize = 6;
const EMPTY: u8 = 0;
const LEVELS: [u8; 6] = [0, 95, 135, 175, 215, 255];

pub(crate) fn encode(source: &ImageSource, window: (u32, u32), cols: u16, rows: u16) -> String {
    let width = usize::from(cols).saturating_mul(CELL_W);
    let height = usize::from(rows).saturating_mul(CELL_H);
    let pixels = palette_pixels(source, window, width, height);
    let used: BTreeSet<u8> = pixels.iter().copied().filter(|reg| *reg != EMPTY).collect();
    let mut seq = String::new();
    let _ = write!(seq, "\x1bPq\"1;1;{width};{height}");
    for reg in used {
        let (r, g, b) = register_rgb(reg);
        let _ = write!(seq, "#{reg};2;{};{};{}", pct(r), pct(g), pct(b));
    }
    for top in (0..height).step_by(BAND) {
        band(&mut seq, &pixels, width, top, height);
    }
    seq.push_str("\x1b\\");
    seq
}

fn palette_pixels(
    source: &ImageSource,
    window: (u32, u32),
    width: usize,
    height: usize,
) -> Vec<u8> {
    let sample = Sample::new(source, window);
    let mut out = Vec::with_capacity(width.saturating_mul(height));
    for y in 0..height {
        for x in 0..width {
            let sx = x.saturating_mul(sample.width) / width.max(1);
            let sy = y.saturating_mul(sample.height) / height.max(1);
            out.push(register(sample.texel(sx, sy)));
        }
    }
    out
}

fn register(px: [u8; 4]) -> u8 {
    let [.., alpha] = px;
    if alpha < 64 {
        return EMPTY;
    }
    crate::terminal::colour::cube(premultiplied(px))
}

fn register_rgb(reg: u8) -> (u8, u8, u8) {
    if reg >= 232 {
        let gray = 8_u8.saturating_add(10 * (reg - 232));
        return (gray, gray, gray);
    }
    let v = reg.saturating_sub(16);
    let level = |i: u8| LEVELS.get(usize::from(i)).copied().unwrap_or_default();
    (level(v / 36), level((v % 36) / 6), level(v % 6))
}

fn pct(value: u8) -> u8 {
    u8::try_from(u32::from(value) * 100 / 255).unwrap_or(100)
}

fn band(seq: &mut String, pixels: &[u8], width: usize, top: usize, height: usize) {
    let view = Band {
        pixels,
        width,
        top,
        height,
    };
    for reg in view.regs() {
        let _ = write!(seq, "#{reg}");
        let mut ch = '?';
        let mut run = 0_usize;
        for x in 0..width {
            let next = view.glyph(x, reg);
            if next == ch {
                run = run.saturating_add(1);
            } else {
                run_out(seq, ch, run);
                ch = next;
                run = 1;
            }
        }
        run_out(seq, ch, run);
        seq.push('$');
    }
    seq.push('-');
}

fn run_out(seq: &mut String, ch: char, count: usize) {
    if count > 3 {
        let _ = write!(seq, "!{count}{ch}");
    } else {
        seq.extend(std::iter::repeat_n(ch, count));
    }
}

struct Band<'a> {
    pixels: &'a [u8],
    width: usize,
    top: usize,
    height: usize,
}

impl Band<'_> {
    fn regs(&self) -> BTreeSet<u8> {
        let mut regs = BTreeSet::new();
        for y in self.top..self.top.saturating_add(BAND).min(self.height) {
            for x in 0..self.width {
                if let Some(&reg) = self
                    .pixels
                    .get(y.saturating_mul(self.width).saturating_add(x))
                    && reg != EMPTY
                {
                    regs.insert(reg);
                }
            }
        }
        regs
    }

    fn glyph(&self, x: usize, reg: u8) -> char {
        let mut bits = 0_u8;
        for dy in 0..BAND {
            let at = self
                .top
                .saturating_add(dy)
                .saturating_mul(self.width)
                .saturating_add(x);
            if self.pixels.get(at) == Some(&reg) {
                bits |= 1 << dy;
            }
        }
        char::from(63_u8.saturating_add(bits))
    }
}
