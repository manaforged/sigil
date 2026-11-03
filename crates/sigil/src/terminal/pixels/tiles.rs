use std::io::{self, Write as _};

use image::ImageEncoder as _;
use image::codecs::png::{CompressionType, FilterType, PngEncoder};
use tiny_skia::Pixmap;

use crate::profile::{self, Phase};
use crate::terminal::graphics::{CHUNK, Mux, base64, write_mux};

const IMAGE: u32 = 0x5167_0001;
/// The side of a tile in pixels; a tile covers whole cells.
const SIDE: u16 = 256;

/// The cell grid of a frame: its size in cells and one cell in pixels.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct Grid {
    pub(super) cols: u16,
    pub(super) rows: u16,
    pub(super) cell: [u16; 2],
}

impl Grid {
    /// One tile in cells as `[cols, rows]`, about `SIDE` pixels on each side.
    pub(super) fn tile(self) -> [u16; 2] {
        [
            (SIDE / self.cell[0].max(1)).max(1),
            (SIDE / self.cell[1].max(1)).max(1),
        ]
    }
}

/// Cell-aligned Kitty images that cover the frame, so a change resends only the tiles it touches.
#[derive(Default)]
pub(super) struct Tiles {
    grid: Option<Grid>,
    sent: u32,
}

impl Tiles {
    /// Sends the tiles of `pixmap` that meet `dirty` (pixel rects as `[x, y, w, h]`), or every tile when the grid changed.
    pub(super) fn send(
        &mut self,
        grid: Grid,
        pixmap: &Pixmap,
        dirty: Option<&[[usize; 4]]>,
        mux: Option<Mux>,
    ) -> io::Result<()> {
        let encode = profile::span(Phase::RenderEncode);
        let mut out = Vec::new();
        let dirty = if self.grid == Some(grid) { dirty } else { None };
        if dirty.is_none() {
            self.erase(&mut out, mux)?;
        }
        out.extend_from_slice(b"\x1b[?2026h\x1b7");
        let mut index = 0;
        let [cols, rows] = grid.tile();
        for top in (0..grid.rows).step_by(usize::from(rows)) {
            for left in (0..grid.cols).step_by(usize::from(cols)) {
                let tile = Tile {
                    left,
                    top,
                    cols: cols.min(grid.cols - left),
                    rows: rows.min(grid.rows - top),
                };
                let id = IMAGE + index;
                index += 1;
                let rect = tile.pixels(grid.cell);
                if dirty.is_some_and(|rects| !rects.iter().any(|r| meets(*r, rect))) {
                    continue;
                }
                tile.write(&mut out, id, &png(pixmap, rect)?, mux)?;
            }
        }
        out.extend_from_slice(b"\x1b8\x1b[?2026l");
        self.sent = index;
        self.grid = Some(grid);
        drop(encode);
        let _write = profile::span(Phase::TuiWrite);
        let mut stdout = io::stdout().lock();
        stdout.write_all(&out)?;
        stdout.flush()
    }

    /// Deletes every tile and forgets the last frame.
    pub(super) fn clear(&mut self, mux: Option<Mux>) -> io::Result<()> {
        let mut out = Vec::new();
        self.erase(&mut out, mux)?;
        let mut stdout = io::stdout().lock();
        stdout.write_all(&out)?;
        stdout.flush()
    }

    fn erase(&mut self, out: &mut Vec<u8>, mux: Option<Mux>) -> io::Result<()> {
        for index in 0..self.sent {
            let id = IMAGE + index;
            write_mux(out, &format!("\x1b_Ga=d,d=I,i={id},q=2\x1b\\"), mux)?;
        }
        self.sent = 0;
        self.grid = None;
        Ok(())
    }
}

struct Tile {
    left: u16,
    top: u16,
    cols: u16,
    rows: u16,
}

impl Tile {
    fn pixels(&self, [width, height]: [u16; 2]) -> [usize; 4] {
        let (width, height) = (usize::from(width), usize::from(height));
        [
            usize::from(self.left) * width,
            usize::from(self.top) * height,
            usize::from(self.cols) * width,
            usize::from(self.rows) * height,
        ]
    }

    fn write(&self, out: &mut Vec<u8>, id: u32, png: &[u8], mux: Option<Mux>) -> io::Result<()> {
        write!(out, "\x1b[{};{}H", self.top + 1, self.left + 1)?;
        let data = base64(png);
        let chunks: Vec<&[u8]> = data.as_bytes().chunks(CHUNK).collect();
        for (index, chunk) in chunks.iter().enumerate() {
            let more = u8::from(index + 1 < chunks.len());
            let head = if index == 0 {
                format!(
                    "a=T,f=100,i={id},p=1,q=2,C=1,z=1,c={},r={},m={more}",
                    self.cols, self.rows
                )
            } else {
                format!("m={more}")
            };
            let body = std::str::from_utf8(chunk).unwrap_or_default();
            write_mux(out, &format!("\x1b_G{head};{body}\x1b\\"), mux)?;
        }
        Ok(())
    }
}

const fn meets([ax, ay, aw, ah]: [usize; 4], [bx, by, bw, bh]: [usize; 4]) -> bool {
    ax < bx + bw && bx < ax + aw && ay < by + bh && by < ay + ah
}

fn png(pixmap: &Pixmap, [x, y, w, h]: [usize; 4]) -> io::Result<Vec<u8>> {
    let stride = pixmap.width() as usize;
    let pixels = pixmap.pixels();
    let mut rgba = Vec::with_capacity(w * h * 4);
    for row in y..y + h {
        for px in &pixels[row * stride + x..row * stride + x + w] {
            let c = px.demultiply();
            rgba.extend_from_slice(&[c.red(), c.green(), c.blue(), c.alpha()]);
        }
    }
    let mut png = Vec::new();
    let size = |n: usize| u32::try_from(n).map_err(io::Error::other);
    PngEncoder::new_with_quality(&mut png, CompressionType::Fast, FilterType::Adaptive)
        .write_image(&rgba, size(w)?, size(h)?, image::ExtendedColorType::Rgba8)
        .map_err(io::Error::other)?;
    Ok(png)
}
