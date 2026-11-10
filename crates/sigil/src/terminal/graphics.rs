use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::io::{self, Write};

use crate::kernel::ImageSource;
use crossterm::{cursor, queue};

mod detect;
mod encode;
mod sixel;

pub(crate) use detect::detect_mux;
pub use detect::{Mux, Protocol, detect};
pub use encode::base64;
#[cfg(test)]
pub(crate) use encode::blocks_window;
#[cfg(any(test, feature = "kittest"))]
pub use encode::{blocks, png, sextants};
pub(crate) use encode::{png_window, sextant, sextants_window};

pub(super) const CHUNK: usize = 4096;
const CACHE: usize = 16;

/// An image placed on the cell grid.
#[derive(Clone, Debug)]
pub struct Placement {
    /// The col.
    pub col: u16,
    /// The row.
    pub row: u16,
    /// The cols.
    pub cols: u16,
    /// The rows.
    pub rows: u16,
    /// The mark col.
    pub mark_col: u16,
    /// The mark row.
    pub mark_row: u16,
    /// The span cols.
    pub span_cols: u16,
    /// The span rows.
    pub span_rows: u16,
    /// The source.
    pub source: ImageSource,
    /// The crop, if any.
    pub crop: Option<(u32, u32)>,
}

impl PartialEq for Placement {
    fn eq(&self, other: &Self) -> bool {
        self.same(other)
    }
}

impl Eq for Placement {}

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
struct ImageAnchor {
    id: u64,
    cols: u16,
    rows: u16,
    crop: Option<(u32, u32)>,
}

impl ImageAnchor {
    const fn of(place: &Placement) -> Self {
        Self {
            id: place.source.id(),
            cols: place.span_cols,
            rows: place.span_rows,
            crop: place.crop,
        }
    }
}

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
struct Same {
    col: u16,
    row: u16,
    anchor: ImageAnchor,
}

impl Same {
    const fn of(place: &Placement) -> Self {
        Self {
            col: place.col,
            row: place.row,
            anchor: ImageAnchor::of(place),
        }
    }
}

impl Placement {
    /// Returns whether the two placements show the same image in the same place.
    #[must_use]
    pub fn same(&self, other: &Self) -> bool {
        self.col == other.col && self.row == other.row && self.anchored(other)
    }

    fn anchored(&self, other: &Self) -> bool {
        self.span_cols == other.span_cols
            && self.span_rows == other.span_rows
            && self.source.id() == other.source.id()
            && self.crop == other.crop
    }

    fn window(&self) -> (u32, u32) {
        self.crop.unwrap_or_else(|| (0, self.source.height()))
    }
}

#[derive(Clone, Copy)]
pub(crate) struct ImageFrame<'a> {
    pub(crate) next: &'a [Placement],
    pub(crate) rewritten: &'a [bool],
    pub(crate) redraw: bool,
}

#[derive(Default)]
pub(crate) struct ImagePainter {
    sent: HashSet<u32>,
    shown: Vec<Placement>,
    encoded: HashMap<(u64, u32, u32), String>,
    sixel: HashMap<(u64, u32, u32, u16, u16), String>,
    cell: Option<[u16; 2]>,
}

impl ImagePainter {
    pub(crate) fn emit(
        &mut self,
        out: &mut impl Write,
        caps: crate::terminal::caps::Caps,
        frame: ImageFrame<'_>,
    ) -> io::Result<()> {
        let Some(protocol) = caps.graphics else {
            return Ok(());
        };
        if self.cell != caps.cell {
            self.cell = caps.cell;
            self.sixel.clear();
        }
        self.emit_mux(out, protocol, caps.mux, frame)
    }

    fn emit_mux(
        &mut self,
        out: &mut impl Write,
        protocol: Protocol,
        mux: Option<Mux>,
        frame: ImageFrame<'_>,
    ) -> io::Result<()> {
        let ImageFrame {
            next,
            rewritten,
            redraw,
        } = frame;
        if mux == Some(Mux::Screen) && matches!(protocol, Protocol::Kitty | Protocol::Sixel) {
            self.shown = next.to_vec();
            return Ok(());
        }
        let shown_anchors: HashSet<ImageAnchor> = self.shown.iter().map(ImageAnchor::of).collect();
        let shown_same: HashSet<Same> = self.shown.iter().map(Same::of).collect();
        let dirty: HashSet<usize> = rewritten
            .iter()
            .enumerate()
            .filter(|(_, changed)| **changed)
            .map(|(row, _)| row)
            .collect();
        if protocol == Protocol::Kitty {
            self.delete_stale(out, mux, next)?;
        }
        for place in next {
            let kept = if protocol == Protocol::Kitty {
                shown_anchors.contains(&ImageAnchor::of(place))
            } else {
                shown_same.contains(&Same::of(place))
            };
            let mut touched = false;
            if protocol != Protocol::Kitty {
                let start = usize::from(place.row);
                let end = start + usize::from(place.rows);
                for row in start..end {
                    if dirty.contains(&row) {
                        touched = true;
                        break;
                    }
                }
            }
            if kept && !touched && !redraw {
                continue;
            }
            self.place(out, protocol, mux, place)?;
        }
        self.shown = next.to_vec();
        Ok(())
    }

    fn delete_stale(
        &mut self,
        out: &mut impl Write,
        mux: Option<Mux>,
        next: &[Placement],
    ) -> io::Result<()> {
        let live: HashSet<u64> = next.iter().map(|place| place.source.id()).collect();
        let anchors: HashSet<ImageAnchor> = next.iter().map(ImageAnchor::of).collect();
        for old in &self.shown {
            let id = kitty_id(&old.source);
            if live.contains(&old.source.id()) {
                if !anchors.contains(&ImageAnchor::of(old)) {
                    let pid = kitty_virtual_id(old);
                    write_mux(out, &format!("\x1b_Ga=d,d=p,i={id},p={pid},q=2\x1b\\"), mux)?;
                }
            } else {
                write_mux(out, &format!("\x1b_Ga=d,d=I,i={id},q=2\x1b\\"), mux)?;
                self.sent.remove(&id);
            }
        }
        Ok(())
    }

    fn place(
        &mut self,
        out: &mut impl Write,
        protocol: Protocol,
        mux: Option<Mux>,
        place: &Placement,
    ) -> io::Result<()> {
        let (top, height) = place.window();
        let (cols, rows) = (place.cols, place.rows);
        let seq = match protocol {
            Protocol::Kitty => {
                let (span_cols, span_rows) = (place.span_cols, place.span_rows);
                let id = kitty_id(&place.source);
                if self.sent.insert(id) {
                    kitty_transmit(out, id, &place.source, mux)?;
                }
                let window = place
                    .crop
                    .map(|(row, span)| format!(",x=0,y={row},w={},h={span}", place.source.width()))
                    .unwrap_or_default();
                let pid = kitty_virtual_id(place);
                Cow::Owned(format!(
                    "\x1b_Ga=p,U=1,i={id},p={pid},c={span_cols},r={span_rows}{window},q=2\x1b\\"
                ))
            }
            Protocol::Iterm2 => {
                let data = cached(&mut self.encoded, (place.source.id(), top, height), || {
                    base64(&png_window(&place.source, top, height))
                });
                Cow::Owned(format!(
                    "\x1b]1337;File=inline=1;width={cols};height={rows};preserveAspectRatio=0:{data}\x07"
                ))
            }
            Protocol::Sixel => {
                let cell = self.cell;
                Cow::Borrowed(cached(
                    &mut self.sixel,
                    (place.source.id(), top, height, cols, rows),
                    || sixel::encode(&place.source, (top, height), (cols, rows), cell),
                ))
            }
        };
        if protocol != Protocol::Kitty {
            queue!(out, cursor::MoveTo(place.col, place.row))?;
        }
        write_mux(out, &seq, mux)
    }
}

fn cached<K: Hash + Eq>(
    map: &mut HashMap<K, String>,
    key: K,
    make: impl FnOnce() -> String,
) -> &str {
    if map.len() >= CACHE && !map.contains_key(&key) {
        map.clear();
    }
    map.entry(key).or_insert_with(make)
}

pub(crate) fn kitty_id(source: &ImageSource) -> u32 {
    u32::try_from(source.id() % u64::from(u32::MAX - 1)).unwrap_or_default() + 1
}

fn kitty_virtual_id(place: &Placement) -> u32 {
    let mut hash = place.source.id();
    for part in [u64::from(place.span_cols), u64::from(place.span_rows)] {
        hash = (hash ^ part).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    }
    if let Some((row, span)) = place.crop {
        for part in [u64::from(row), u64::from(span)] {
            hash = (hash ^ part).wrapping_mul(0x9E37_79B9_7F4A_7C15);
        }
    }
    u32::try_from(hash % u64::from(u32::MAX - 1)).unwrap_or_default() + 1
}

fn kitty_transmit(
    out: &mut impl Write,
    id: u32,
    source: &ImageSource,
    mux: Option<Mux>,
) -> io::Result<()> {
    let data = base64(source.rgba());
    let chunks: Vec<&[u8]> = data.as_bytes().chunks(CHUNK).collect();
    for (index, chunk) in chunks.iter().enumerate() {
        let more = u8::from(index + 1 < chunks.len());
        let head = if index == 0 {
            format!(
                "a=t,f=32,s={},v={},i={id},q=2,m={more}",
                source.width(),
                source.height()
            )
        } else {
            format!("m={more}")
        };
        let body = std::str::from_utf8(chunk).unwrap_or_default();
        write_mux(out, &format!("\x1b_G{head};{body}\x1b\\"), mux)?;
    }
    Ok(())
}

pub(crate) fn write_mux(out: &mut impl Write, seq: &str, mux: Option<Mux>) -> io::Result<()> {
    match mux {
        None => out.write_all(seq.as_bytes()),
        Some(Mux::Tmux) => write!(out, "\x1bPtmux;{}\x1b\\", seq.replace('\x1b', "\x1b\x1b")),
        Some(Mux::Screen) => write!(out, "\x1bP{seq}\x1b\\"),
    }
}

#[cfg(test)]
#[path = "graphics_tests.rs"]
mod tests;
