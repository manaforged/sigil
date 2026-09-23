use std::collections::BTreeSet;
use std::fmt::Write as _;

use sigil::kernel::ImageSource;
use sigil::style::Rgba8;

const CELL_W: usize = 8;
const CELL_H: usize = 16;
const BAND: usize = 6;
const EMPTY: u8 = 0;
const LEVELS: [u8; 6] = [0, 95, 135, 175, 215, 255];

pub(crate) fn encode(source: &ImageSource, window: (u32, u32), cols: u16, rows: u16) -> String {
    let width = usize::from(cols).saturating_mul(CELL_W);
    let height = usize::from(rows).saturating_mul(CELL_H);
    let pixels = palette_pixels(source, window, width, height);
    let mut used = BTreeSet::new();
    for &reg in &pixels {
        if reg != EMPTY {
            used.insert(reg);
        }
    }
    let mut seq = String::new();
    let _ = write!(seq, "\x1bPq\"1;1;{width};{height}");
    for reg in &used {
        let (r, g, b) = register_rgb(*reg);
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
    let sw = source.width() as usize;
    let full = source.height() as usize;
    let top = (window.0 as usize).min(full);
    let wh = (window.1 as usize).clamp(1, full.saturating_sub(top).max(1));
    let rgba = source.rgba();
    let mut out = vec![EMPTY; width.saturating_mul(height)];
    for y in 0..height {
        for x in 0..width {
            let sx = x.saturating_mul(sw) / width.max(1);
            let sy = top.saturating_add(y.saturating_mul(wh) / height.max(1));
            let at = (sy.min(full.saturating_sub(1)) * sw + sx.min(sw.saturating_sub(1))) * 4;
            let px = rgba
                .get(at..at + 4)
                .and_then(|p| <[u8; 4]>::try_from(p).ok())
                .unwrap_or_default();
            if let Some(slot) = out.get_mut(y.saturating_mul(width).saturating_add(x)) {
                *slot = register(px);
            }
        }
    }
    out
}

fn register(px: [u8; 4]) -> u8 {
    let [r, g, b, a] = px;
    if a < 64 {
        return EMPTY;
    }
    let alpha = u32::from(a);
    let over = |channel: u8| u8::try_from(u32::from(channel) * alpha / 255).unwrap_or(u8::MAX);
    crate::write::cube(Rgba8 {
        r: over(r),
        g: over(g),
        b: over(b),
        a: 255,
    })
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
    if count == 0 {
        return;
    }
    if count > 3 {
        let _ = write!(seq, "!{count}{ch}");
    } else {
        for _ in 0..count {
            seq.push(ch);
        }
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
