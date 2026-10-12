use crate::kernel::ImageSource;
use crate::style::Rgba8;

const STORED: usize = 65_535;

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
    cells(source, window, (cols, rows), |sample, col, row| {
        let x = col * sample.width / cols;
        let y = |half: usize| (row * 2 + half) * sample.height / (rows * 2);
        (sample.pixel(x, y(0)), sample.pixel(x, y(1)))
    })
}

#[must_use]
pub fn sextants(source: &ImageSource, cols: usize, rows: usize) -> Vec<Vec<(char, Rgba8, Rgba8)>> {
    sextants_window(source, (0, source.height()), cols, rows)
}

#[must_use]
pub fn sextants_window(
    source: &ImageSource,
    window: (u32, u32),
    cols: usize,
    rows: usize,
) -> Vec<Vec<(char, Rgba8, Rgba8)>> {
    cells(source, window, (cols, rows), |sample, col, row| {
        cell_glyph(std::array::from_fn(|index| {
            let x = (col * 2 + index % 2) * sample.width / (cols * 2);
            let y = (row * 3 + index / 2) * sample.height / (rows * 3);
            sample.pixel(x, y)
        }))
    })
}

fn cells<T>(
    source: &ImageSource,
    window: (u32, u32),
    (cols, rows): (usize, usize),
    at: impl Fn(&Sample<'_>, usize, usize) -> T,
) -> Vec<Vec<T>> {
    let sample = Sample::new(source, window);
    if cols == 0 || rows == 0 || sample.width == 0 || sample.full == 0 {
        return Vec::new();
    }
    (0..rows)
        .map(|row| (0..cols).map(|col| at(&sample, col, row)).collect())
        .collect()
}

pub(super) struct Sample<'a> {
    rgba: &'a [u8],
    pub(super) width: usize,
    full: usize,
    top: usize,
    pub(super) height: usize,
}

impl<'a> Sample<'a> {
    pub(super) fn new(source: &'a ImageSource, window: (u32, u32)) -> Self {
        let full = source.height() as usize;
        let top = (window.0 as usize).min(full);
        Self {
            rgba: source.rgba(),
            width: source.width() as usize,
            full,
            top,
            height: (window.1 as usize).clamp(1, full.saturating_sub(top).max(1)),
        }
    }

    pub(super) fn texel(&self, x: usize, y: usize) -> [u8; 4] {
        let line = self.top.saturating_add(y).min(self.full.saturating_sub(1));
        let at = (line * self.width + x.min(self.width.saturating_sub(1))) * 4;
        self.rgba
            .get(at..at + 4)
            .and_then(|px| <[u8; 4]>::try_from(px).ok())
            .unwrap_or_default()
    }

    fn pixel(&self, x: usize, y: usize) -> Rgba8 {
        premultiplied(self.texel(x, y))
    }
}

pub(super) fn premultiplied([r, g, b, a]: [u8; 4]) -> Rgba8 {
    let alpha = u32::from(a);
    let over = |channel: u8| u8::try_from(u32::from(channel) * alpha / 255).unwrap_or(u8::MAX);
    Rgba8 {
        r: over(r),
        g: over(g),
        b: over(b),
        a: 255,
    }
}

fn cell_glyph(px: [Rgba8; 6]) -> (char, Rgba8, Rgba8) {
    let lum = |c: &Rgba8| u32::from(c.r) * 299 + u32::from(c.g) * 587 + u32::from(c.b) * 114;
    let mean = px.iter().map(lum).sum::<u32>() / 6;
    let mask = px.iter().enumerate().fold(0u32, |bits, (index, c)| {
        bits | u32::from(lum(c) >= mean) << index
    });
    let mean_of = |set: bool| {
        let picked = px
            .iter()
            .enumerate()
            .filter(|(index, _)| (mask & (1 << index) != 0) == set);
        let count = u32::try_from(picked.clone().count().max(1)).unwrap_or(1);
        let (r, g, b) = picked.fold((0u32, 0u32, 0u32), |(r, g, b), (_, c)| {
            (r + u32::from(c.r), g + u32::from(c.g), b + u32::from(c.b))
        });
        Rgba8 {
            r: u8::try_from(r / count).unwrap_or(u8::MAX),
            g: u8::try_from(g / count).unwrap_or(u8::MAX),
            b: u8::try_from(b / count).unwrap_or(u8::MAX),
            a: 255,
        }
    };
    (sextant(mask), mean_of(true), mean_of(false))
}

pub(crate) fn sextant(mask: u32) -> char {
    let code = match mask {
        0 => return ' ',
        0x15 => return '▌',
        0x2A => return '▐',
        0x3F => return '█',
        m => 0x1FB00 + m - 1 - u32::from(m > 0x15) - u32::from(m > 0x2A),
    };
    char::from_u32(code).unwrap_or(' ')
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
    out.extend_from_slice(&crc32(crc32(0, &kind), data).to_be_bytes());
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

const CRC_TABLE: [u32; 256] = crc_table();

const fn crc_table() -> [u32; 256] {
    let mut table = [0_u32; 256];
    let mut index = 0_u32;
    let mut rest: &mut [u32] = &mut table;
    while let [slot, tail @ ..] = rest {
        let mut value = index;
        let mut bit = 0;
        while bit < 8 {
            value = if value & 1 == 1 {
                0xedb8_8320 ^ (value >> 1)
            } else {
                value >> 1
            };
            bit += 1;
        }
        *slot = value;
        index += 1;
        rest = tail;
    }
    table
}

fn crc32(start: u32, data: &[u8]) -> u32 {
    let mut crc = !start;
    for byte in data {
        let entry = CRC_TABLE
            .get(((crc ^ u32::from(*byte)) & 0xff) as usize)
            .copied()
            .unwrap_or_default();
        crc = entry ^ (crc >> 8);
    }
    !crc
}
