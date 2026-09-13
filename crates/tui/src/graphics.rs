use std::collections::{HashMap, HashSet};
use std::io::{self, Write};
use std::sync::OnceLock;

use crossterm::{cursor, queue};
use sigil::kernel::ImageSource;
use sigil::style::Rgba8;

const CHUNK: usize = 4096;
const STORED: usize = 65_535;
const CACHE: usize = 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Protocol {
    Kitty,
    Iterm2,
}

pub fn protocol() -> Option<Protocol> {
    static FOUND: OnceLock<Option<Protocol>> = OnceLock::new();
    FOUND.get_or_init(from_env).as_ref().copied()
}

fn from_env() -> Option<Protocol> {
    detect(|key| std::env::var(key).ok())
}

pub fn detect(env: impl Fn(&str) -> Option<String>) -> Option<Protocol> {
    if let Some(forced) = env("SIGIL_GRAPHICS") {
        return match forced.trim().to_ascii_lowercase().as_str() {
            "kitty" => Some(Protocol::Kitty),
            "iterm2" | "iterm" => Some(Protocol::Iterm2),
            _ => None,
        };
    }
    if env("TMUX").is_some() {
        return None;
    }
    let term = env("TERM").unwrap_or_default();
    let program = env("TERM_PROGRAM").unwrap_or_default();
    if env("KITTY_WINDOW_ID").is_some()
        || env("GHOSTTY_RESOURCES_DIR").is_some()
        || env("WEZTERM_EXECUTABLE").is_some()
        || env("KONSOLE_VERSION").is_some()
        || matches!(term.as_str(), "xterm-kitty" | "xterm-ghostty")
        || matches!(program.as_str(), "WezTerm" | "ghostty")
    {
        return Some(Protocol::Kitty);
    }
    if matches!(program.as_str(), "iTerm.app" | "WarpTerminal")
        || env("LC_TERMINAL").as_deref() == Some("iTerm2")
    {
        return Some(Protocol::Iterm2);
    }
    None
}

#[derive(Clone, Debug)]
pub struct Placement {
    pub col: u16,
    pub row: u16,
    pub cols: u16,
    pub rows: u16,
    pub source: ImageSource,
    pub crop: Option<(u32, u32)>,
}

impl PartialEq for Placement {
    fn eq(&self, other: &Self) -> bool {
        self.same(other)
    }
}

impl Eq for Placement {}

impl Placement {
    #[must_use]
    pub fn same(&self, other: &Self) -> bool {
        self.col == other.col
            && self.row == other.row
            && self.cols == other.cols
            && self.rows == other.rows
            && self.source.id() == other.source.id()
            && self.crop == other.crop
    }

    fn window(&self) -> (u32, u32) {
        self.crop.unwrap_or_else(|| (0, self.source.height()))
    }

    fn covers(&self, row: usize) -> bool {
        let top = usize::from(self.row);
        (top..top.saturating_add(usize::from(self.rows))).contains(&row)
    }
}

#[derive(Clone, Copy)]
pub(crate) struct Frame<'a> {
    pub(crate) next: &'a [Placement],
    pub(crate) rewritten: &'a [bool],
    pub(crate) redraw: bool,
}

#[derive(Default)]
pub(crate) struct Painter {
    sent: HashSet<u32>,
    shown: Vec<Placement>,
    encoded: HashMap<(u64, u32, u32), String>,
}

impl Painter {
    pub(crate) fn emit(&mut self, out: &mut impl Write, frame: Frame<'_>) -> io::Result<()> {
        let Some(protocol) = protocol() else {
            return Ok(());
        };
        self.emit_with(out, protocol, frame)
    }

    pub(crate) fn emit_with(
        &mut self,
        out: &mut impl Write,
        protocol: Protocol,
        frame: Frame<'_>,
    ) -> io::Result<()> {
        let Frame {
            next,
            rewritten,
            redraw,
        } = frame;
        if protocol == Protocol::Kitty {
            for old in &self.shown {
                if !next.iter().any(|place| place.same(old)) {
                    kitty_delete(out, kitty_id(&old.source))?;
                }
            }
        }
        for place in next {
            let kept = self.shown.iter().any(|old| old.same(place));
            let touched = rewritten
                .iter()
                .enumerate()
                .any(|(row, changed)| *changed && place.covers(row));
            if kept && !touched && !redraw {
                continue;
            }
            match protocol {
                Protocol::Kitty => {
                    let id = kitty_id(&place.source);
                    if self.sent.insert(id) {
                        kitty_transmit(out, id, &place.source)?;
                    }
                    kitty_place(out, id, place)?;
                }
                Protocol::Iterm2 => {
                    let (top, height) = place.window();
                    let key = (place.source.id(), top, height);
                    if !self.encoded.contains_key(&key) {
                        if self.encoded.len() >= CACHE {
                            self.encoded.clear();
                        }
                        self.encoded
                            .insert(key, base64(&png_window(&place.source, top, height)));
                    }
                    let data = self
                        .encoded
                        .get(&key)
                        .map(String::as_str)
                        .unwrap_or_default();
                    iterm_place(out, data, place)?;
                }
            }
        }
        self.shown = next.to_vec();
        Ok(())
    }
}

fn kitty_id(source: &ImageSource) -> u32 {
    u32::try_from(source.id() % u64::from(u32::MAX - 1)).unwrap_or_default() + 1
}

fn kitty_transmit(out: &mut impl Write, id: u32, source: &ImageSource) -> io::Result<()> {
    let data = base64(source.rgba());
    let chunks: Vec<&[u8]> = data.as_bytes().chunks(CHUNK).collect();
    for (index, chunk) in chunks.iter().enumerate() {
        let more = u8::from(index + 1 < chunks.len());
        if index == 0 {
            write!(
                out,
                "\x1b_Ga=t,f=32,s={},v={},i={id},q=2,m={more};",
                source.width(),
                source.height()
            )?;
        } else {
            write!(out, "\x1b_Gm={more};")?;
        }
        out.write_all(chunk)?;
        out.write_all(b"\x1b\\")?;
    }
    Ok(())
}

fn kitty_place(out: &mut impl Write, id: u32, place: &Placement) -> io::Result<()> {
    queue!(out, cursor::MoveTo(place.col, place.row))?;
    let window = place
        .crop
        .map(|(top, height)| format!(",x=0,y={top},w={},h={height}", place.source.width()))
        .unwrap_or_default();
    write!(
        out,
        "\x1b_Ga=p,i={id},p=1,c={},r={}{window},C=1,q=2\x1b\\",
        place.cols, place.rows
    )
}

fn kitty_delete(out: &mut impl Write, id: u32) -> io::Result<()> {
    write!(out, "\x1b_Ga=d,d=i,i={id},q=2\x1b\\")
}

fn iterm_place(out: &mut impl Write, data: &str, place: &Placement) -> io::Result<()> {
    queue!(out, cursor::MoveTo(place.col, place.row))?;
    write!(
        out,
        "\x1b]1337;File=inline=1;width={};height={};preserveAspectRatio=0:{data}\x07",
        place.cols, place.rows
    )
}

#[must_use]
pub fn blocks(source: &ImageSource, cols: usize, rows: usize) -> Vec<Vec<(Rgba8, Rgba8)>> {
    blocks_window(source, (0, source.height()), cols, rows)
}

#[must_use]
pub fn blocks_window(
    source: &ImageSource,
    window: (u32, u32),
    cols: usize,
    rows: usize,
) -> Vec<Vec<(Rgba8, Rgba8)>> {
    let width = source.width() as usize;
    let full = source.height() as usize;
    let top = (window.0 as usize).min(full);
    let height = (window.1 as usize).clamp(1, full.saturating_sub(top).max(1));
    if cols == 0 || rows == 0 || width == 0 || full == 0 {
        return Vec::new();
    }
    let rgba = source.rgba();
    let pixel = |x: usize, y: usize| {
        let line = top.saturating_add(y).min(full.saturating_sub(1));
        let at = (line * width + x.min(width - 1)) * 4;
        let [r, g, b, a] = rgba
            .get(at..at + 4)
            .and_then(|px| <[u8; 4]>::try_from(px).ok())
            .unwrap_or_default();
        let alpha = u32::from(a);
        let over = |channel: u8| u8::try_from(u32::from(channel) * alpha / 255).unwrap_or(u8::MAX);
        Rgba8 {
            r: over(r),
            g: over(g),
            b: over(b),
            a: 255,
        }
    };
    (0..rows)
        .map(|row| {
            (0..cols)
                .map(|col| {
                    let x = col * width / cols;
                    let upper = row * 2 * height / (rows * 2);
                    let lower = (row * 2 + 1) * height / (rows * 2);
                    (pixel(x, upper), pixel(x, lower))
                })
                .collect()
        })
        .collect()
}

#[must_use]
pub fn base64(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let word = chunk.iter().enumerate().fold(0_u32, |word, (index, byte)| {
            word | u32::from(*byte) << (16 - 8 * index)
        });
        for index in 0..4 {
            if index <= chunk.len() {
                let six = (word >> (18 - 6 * index)) & 63;
                out.push(char::from(TABLE.get(six as usize).copied().unwrap_or(b'=')));
            } else {
                out.push('=');
            }
        }
    }
    out
}

#[must_use]
pub fn png(source: &ImageSource) -> Vec<u8> {
    png_window(source, 0, source.height())
}

#[must_use]
pub fn png_window(source: &ImageSource, top: u32, height: u32) -> Vec<u8> {
    let width = source.width();
    let top = top.min(source.height());
    let height = height.clamp(1, source.height().saturating_sub(top).max(1));
    let stride = width as usize * 4;
    let mut raw = Vec::with_capacity((stride + 1) * height as usize);
    for row in source
        .rgba()
        .chunks(stride.max(1))
        .skip(top as usize)
        .take(height as usize)
    {
        raw.push(0);
        raw.extend_from_slice(row);
    }
    let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut header = Vec::with_capacity(13);
    header.extend_from_slice(&width.to_be_bytes());
    header.extend_from_slice(&height.to_be_bytes());
    header.extend_from_slice(&[8, 6, 0, 0, 0]);
    chunk(&mut out, *b"IHDR", &header);
    chunk(&mut out, *b"IDAT", &zlib_stored(&raw));
    chunk(&mut out, *b"IEND", &[]);
    out
}

fn chunk(out: &mut Vec<u8>, kind: [u8; 4], data: &[u8]) {
    out.extend_from_slice(&u32::try_from(data.len()).unwrap_or(u32::MAX).to_be_bytes());
    out.extend_from_slice(&kind);
    out.extend_from_slice(data);
    let mut crc = crc32(0, &kind);
    crc = crc32(crc, data);
    out.extend_from_slice(&crc.to_be_bytes());
}

fn zlib_stored(data: &[u8]) -> Vec<u8> {
    let mut out = vec![0x78, 0x01];
    if data.is_empty() {
        out.extend_from_slice(&[1, 0, 0, 0xff, 0xff]);
    }
    let blocks = data.chunks(STORED).count();
    for (index, block) in data.chunks(STORED).enumerate() {
        out.push(u8::from(index + 1 == blocks));
        let len = u16::try_from(block.len()).unwrap_or(u16::MAX);
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(&(!len).to_le_bytes());
        out.extend_from_slice(block);
    }
    out.extend_from_slice(&adler32(data).to_be_bytes());
    out
}

fn adler32(data: &[u8]) -> u32 {
    let (mut a, mut b) = (1_u32, 0_u32);
    for byte in data {
        a = (a + u32::from(*byte)) % 65_521;
        b = (b + a) % 65_521;
    }
    b << 16 | a
}

fn crc32(start: u32, data: &[u8]) -> u32 {
    static TABLE: OnceLock<[u32; 256]> = OnceLock::new();
    let table = TABLE.get_or_init(|| {
        let mut table = [0_u32; 256];
        for (index, slot) in table.iter_mut().enumerate() {
            let mut value = u32::try_from(index).unwrap_or_default();
            for _ in 0..8 {
                value = if value & 1 == 1 {
                    0xedb8_8320 ^ (value >> 1)
                } else {
                    value >> 1
                };
            }
            *slot = value;
        }
        table
    });
    let mut crc = !start;
    for byte in data {
        let entry = table
            .get(((crc ^ u32::from(*byte)) & 0xff) as usize)
            .copied()
            .unwrap_or_default();
        crc = entry ^ (crc >> 8);
    }
    !crc
}

#[cfg(test)]
#[path = "graphics_tests.rs"]
mod tests;
