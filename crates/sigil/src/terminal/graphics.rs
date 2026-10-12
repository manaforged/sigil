use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::io::{self, Write};

use crate::kernel::ImageSource;
use crate::terminal::caps::{Known, known_env};
use crossterm::{cursor, queue};

mod encode;
mod sixel;

pub(crate) use encode::sextant;
pub use encode::{base64, blocks, blocks_window, png, png_window, sextants, sextants_window};

const CHUNK: usize = 4096;
const CACHE: usize = 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Protocol {
    Kitty,
    Iterm2,
    Sixel,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Mux {
    Tmux,
    Screen,
}

#[must_use]
pub fn detect_mux(env: impl Fn(&str) -> Option<String>) -> Option<Mux> {
    if env("TMUX").is_some() {
        Some(Mux::Tmux)
    } else if env("STY").is_some() {
        Some(Mux::Screen)
    } else {
        None
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Forced {
    Off,
    On(Protocol),
}

impl std::str::FromStr for Forced {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "none" => Ok(Self::Off),
            "kitty" => Ok(Self::On(Protocol::Kitty)),
            "iterm2" | "iterm" => Ok(Self::On(Protocol::Iterm2)),
            "sixel" => Ok(Self::On(Protocol::Sixel)),
            _ => Err(()),
        }
    }
}

pub(crate) fn forced_graphics(env: &impl Fn(&str) -> Option<String>) -> Option<Forced> {
    env("SIGIL_GRAPHICS")?.parse().ok()
}

pub fn detect(env: impl Fn(&str) -> Option<String>) -> Option<Protocol> {
    match forced_graphics(&env) {
        Some(Forced::Off) => return None,
        Some(Forced::On(protocol)) => return Some(protocol),
        None => {}
    }
    let term = env("TERM").unwrap_or_default();
    let program = env("TERM_PROGRAM").unwrap_or_default();
    if env("KONSOLE_VERSION").is_some() || matches!(term.as_str(), "xterm-kitty" | "xterm-ghostty")
    {
        return Some(Protocol::Kitty);
    }
    match known_env(&env) {
        Some(Known::Kitty | Known::Ghostty) => return Some(Protocol::Kitty),
        Some(Known::WezTerm | Known::Iterm) => return Some(Protocol::Iterm2),
        _ => {}
    }
    if program == "WarpTerminal" {
        return Some(Protocol::Iterm2);
    }
    if sixel_capable(&term, &program) {
        return Some(Protocol::Sixel);
    }
    None
}

fn sixel_capable(term: &str, program: &str) -> bool {
    let base = term.split('-').next().unwrap_or_default();
    term.contains("sixel")
        || matches!(base, "foot" | "contour" | "mlterm" | "yaft" | "rio")
        || ["contour", "mintty", "mlterm", "rio", "foot"]
            .iter()
            .any(|name| program.eq_ignore_ascii_case(name))
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
    sixel: HashMap<(u64, u32, u32, u16, u16), String>,
}

impl Painter {
    pub(crate) fn emit(
        &mut self,
        out: &mut impl Write,
        caps: crate::terminal::caps::Caps,
        frame: Frame<'_>,
    ) -> io::Result<()> {
        let Some(protocol) = caps.graphics else {
            return Ok(());
        };
        self.emit_mux(out, protocol, caps.mux, frame)
    }

    fn emit_mux(
        &mut self,
        out: &mut impl Write,
        protocol: Protocol,
        mux: Option<Mux>,
        frame: Frame<'_>,
    ) -> io::Result<()> {
        let Frame {
            next,
            rewritten,
            redraw,
        } = frame;
        if mux == Some(Mux::Screen) && matches!(protocol, Protocol::Kitty | Protocol::Sixel) {
            self.shown = next.to_vec();
            return Ok(());
        }
        if protocol == Protocol::Kitty {
            for old in &self.shown {
                let id = kitty_id(&old.source);
                if next.iter().any(|place| kitty_id(&place.source) == id) {
                    if !next.iter().any(|place| place.same(old)) {
                        let pid = kitty_place_id(old);
                        write_mux(out, &format!("\x1b_Ga=d,d=p,i={id},p={pid},q=2\x1b\\"), mux)?;
                    }
                } else {
                    write_mux(out, &format!("\x1b_Ga=d,d=I,i={id},q=2\x1b\\"), mux)?;
                    self.sent.remove(&id);
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
            self.place(out, protocol, mux, place)?;
        }
        self.shown = next.to_vec();
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
                let id = kitty_id(&place.source);
                if self.sent.insert(id) {
                    kitty_transmit(out, id, &place.source, mux)?;
                }
                let window = place
                    .crop
                    .map(|(row, span)| format!(",x=0,y={row},w={},h={span}", place.source.width()))
                    .unwrap_or_default();
                let pid = kitty_place_id(place);
                Cow::Owned(format!(
                    "\x1b_Ga=p,i={id},p={pid},c={cols},r={rows}{window},C=1,q=2\x1b\\"
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
            Protocol::Sixel => Cow::Borrowed(cached(
                &mut self.sixel,
                (place.source.id(), top, height, cols, rows),
                || sixel::encode(&place.source, (top, height), cols, rows),
            )),
        };
        queue!(out, cursor::MoveTo(place.col, place.row))?;
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

fn kitty_id(source: &ImageSource) -> u32 {
    u32::try_from(source.id() % u64::from(u32::MAX - 1)).unwrap_or_default() + 1
}

fn kitty_place_id(place: &Placement) -> u32 {
    let (top, height) = place.window();
    let mut hash = u64::from(place.col);
    for part in [
        u64::from(place.row),
        u64::from(place.cols),
        u64::from(place.rows),
        u64::from(top),
        u64::from(height),
    ] {
        hash = (hash ^ part).wrapping_mul(0x9E37_79B9_7F4A_7C15);
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
