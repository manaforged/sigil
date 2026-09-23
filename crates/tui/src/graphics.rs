use std::collections::{HashMap, HashSet};
use std::io::{self, Write};
use std::sync::OnceLock;

use crossterm::{cursor, queue};
use sigil::kernel::ImageSource;

mod encode;
mod sixel;

pub use encode::{base64, blocks, blocks_window, png, png_window, sextants, sextants_window};

const CHUNK: usize = 4096;
const CACHE: usize = 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Protocol {
    Kitty,
    Iterm2,
    Sixel,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mux {
    Tmux,
    Screen,
}

#[must_use]
pub fn protocol() -> Option<Protocol> {
    static FOUND: OnceLock<Option<Protocol>> = OnceLock::new();
    FOUND.get_or_init(from_env).as_ref().copied()
}

fn from_env() -> Option<Protocol> {
    detect(|key| std::env::var(key).ok())
}

#[must_use]
pub fn mux() -> Option<Mux> {
    static FOUND: OnceLock<Option<Mux>> = OnceLock::new();
    FOUND
        .get_or_init(|| detect_mux(|key| std::env::var(key).ok()))
        .as_ref()
        .copied()
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

pub fn detect(env: impl Fn(&str) -> Option<String>) -> Option<Protocol> {
    if let Some(forced) = env("SIGIL_GRAPHICS") {
        return match forced.trim().to_ascii_lowercase().as_str() {
            "kitty" => Some(Protocol::Kitty),
            "iterm2" | "iterm" => Some(Protocol::Iterm2),
            "sixel" => Some(Protocol::Sixel),
            _ => None,
        };
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
    pub(crate) fn emit(&mut self, out: &mut impl Write, frame: Frame<'_>) -> io::Result<()> {
        let Some(protocol) = protocol() else {
            return Ok(());
        };
        self.emit_mux(out, protocol, mux(), frame)
    }

    #[cfg(test)]
    pub(crate) fn emit_with(
        &mut self,
        out: &mut impl Write,
        protocol: Protocol,
        frame: Frame<'_>,
    ) -> io::Result<()> {
        self.emit_mux(out, protocol, None, frame)
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
                        kitty_unplace(out, id, kitty_place_id(old), mux)?;
                    }
                } else {
                    kitty_delete(out, id, mux)?;
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
        match protocol {
            Protocol::Kitty => {
                let id = kitty_id(&place.source);
                if self.sent.insert(id) {
                    kitty_transmit(out, id, &place.source, mux)?;
                }
                kitty_place(out, id, place, mux)
            }
            Protocol::Iterm2 => iterm_place(out, self.cached(place), place, mux),
            Protocol::Sixel => {
                let (top, height) = place.window();
                let key = (place.source.id(), top, height, place.cols, place.rows);
                if !self.sixel.contains_key(&key) {
                    if self.sixel.len() >= CACHE {
                        self.sixel.clear();
                    }
                    self.sixel.insert(
                        key,
                        sixel::encode(&place.source, (top, height), place.cols, place.rows),
                    );
                }
                let data = self.sixel.get(&key).map(String::as_str).unwrap_or_default();
                sixel_place(out, data, place, mux)
            }
        }
    }

    fn cached(&mut self, place: &Placement) -> &str {
        let (top, height) = place.window();
        let key = (place.source.id(), top, height);
        if !self.encoded.contains_key(&key) {
            if self.encoded.len() >= CACHE {
                self.encoded.clear();
            }
            self.encoded
                .insert(key, base64(&png_window(&place.source, top, height)));
        }
        self.encoded
            .get(&key)
            .map(String::as_str)
            .unwrap_or_default()
    }
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
        let mut seq = if index == 0 {
            format!(
                "\x1b_Ga=t,f=32,s={},v={},i={id},q=2,m={more};",
                source.width(),
                source.height()
            )
        } else {
            format!("\x1b_Gm={more};")
        };
        for &byte in *chunk {
            seq.push(char::from(byte));
        }
        seq.push_str("\x1b\\");
        write_mux(out, &seq, mux)?;
    }
    Ok(())
}

fn kitty_place(
    out: &mut impl Write,
    id: u32,
    place: &Placement,
    mux: Option<Mux>,
) -> io::Result<()> {
    queue!(out, cursor::MoveTo(place.col, place.row))?;
    let window = place
        .crop
        .map(|(top, height)| format!(",x=0,y={top},w={},h={height}", place.source.width()))
        .unwrap_or_default();
    let pid = kitty_place_id(place);
    let seq = format!(
        "\x1b_Ga=p,i={id},p={pid},c={},r={}{window},C=1,q=2\x1b\\",
        place.cols, place.rows
    );
    write_mux(out, &seq, mux)
}

fn kitty_unplace(out: &mut impl Write, id: u32, pid: u32, mux: Option<Mux>) -> io::Result<()> {
    write_mux(out, &format!("\x1b_Ga=d,d=p,i={id},p={pid},q=2\x1b\\"), mux)
}

fn kitty_delete(out: &mut impl Write, id: u32, mux: Option<Mux>) -> io::Result<()> {
    write_mux(out, &format!("\x1b_Ga=d,d=I,i={id},q=2\x1b\\"), mux)
}

fn iterm_place(
    out: &mut impl Write,
    data: &str,
    place: &Placement,
    mux: Option<Mux>,
) -> io::Result<()> {
    queue!(out, cursor::MoveTo(place.col, place.row))?;
    let seq = format!(
        "\x1b]1337;File=inline=1;width={};height={};preserveAspectRatio=0:{data}\x07",
        place.cols, place.rows
    );
    write_mux(out, &seq, mux)
}

fn sixel_place(
    out: &mut impl Write,
    data: &str,
    place: &Placement,
    mux: Option<Mux>,
) -> io::Result<()> {
    queue!(out, cursor::MoveTo(place.col, place.row))?;
    write_mux(out, data, mux)
}

pub(crate) fn write_sequence(out: &mut impl Write, seq: &str) -> io::Result<()> {
    write_mux(out, seq, mux())
}

fn write_mux(out: &mut impl Write, seq: &str, mux: Option<Mux>) -> io::Result<()> {
    match mux {
        None => out.write_all(seq.as_bytes()),
        Some(Mux::Tmux) => write!(out, "\x1bPtmux;{}\x1b\\", seq.replace('\x1b', "\x1b\x1b")),
        Some(Mux::Screen) => write!(out, "\x1bP{seq}\x1b\\"),
    }
}

#[cfg(test)]
#[path = "graphics_tests.rs"]
mod tests;
