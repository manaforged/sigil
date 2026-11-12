use std::num::NonZeroU16;
use std::sync::{LazyLock, PoisonError, RwLock};

use ahash::AHashMap;

const INLINE: usize = 7;
const LONG: u8 = u8::MAX;
const ENTRIES: usize = u16::MAX as usize - 1;
const ENTRY_BYTES: usize = 2048;
const REPLACEMENT: &str = "\u{fffd}";
const ASCII: &str = "\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0a\x0b\x0c\x0d\x0e\x0f\x10\x11\x12\x13\x14\x15\x16\x17\x18\x19\x1a\x1b\x1c\x1d\x1e\x1f\x20\x21\x22\x23\x24\x25\x26\x27\x28\x29\x2a\x2b\x2c\x2d\x2e\x2f\x30\x31\x32\x33\x34\x35\x36\x37\x38\x39\x3a\x3b\x3c\x3d\x3e\x3f\x40\x41\x42\x43\x44\x45\x46\x47\x48\x49\x4a\x4b\x4c\x4d\x4e\x4f\x50\x51\x52\x53\x54\x55\x56\x57\x58\x59\x5a\x5b\x5c\x5d\x5e\x5f\x60\x61\x62\x63\x64\x65\x66\x67\x68\x69\x6a\x6b\x6c\x6d\x6e\x6f\x70\x71\x72\x73\x74\x75\x76\x77\x78\x79\x7a\x7b\x7c\x7d\x7e\x7f";

#[derive(Default)]
struct Table {
    ids: AHashMap<&'static str, u16>,
    texts: Vec<&'static str>,
}

type Shared = LazyLock<RwLock<Table>>;

static GLYPHS: Shared = LazyLock::new(RwLock::default);
static LINKS: Shared = LazyLock::new(RwLock::default);

fn intern(table: &Shared, text: &str) -> Option<u16> {
    let known = |seen: &Table| seen.ids.get(text).copied();
    let read = known(&table.read().unwrap_or_else(PoisonError::into_inner));
    if read.is_some() {
        return read;
    }
    let mut write = table.write().unwrap_or_else(PoisonError::into_inner);
    if let Some(id) = known(&write) {
        return Some(id);
    }
    if write.texts.len() >= ENTRIES || text.len() > ENTRY_BYTES {
        return None;
    }
    let id = u16::try_from(write.texts.len()).ok()?;
    let kept: &'static str = Box::leak(Box::from(text));
    write.texts.push(kept);
    write.ids.insert(kept, id);
    drop(write);
    Some(id)
}

fn resolve(table: &Shared, id: u16) -> &'static str {
    table
        .read()
        .unwrap_or_else(PoisonError::into_inner)
        .texts
        .get(usize::from(id))
        .copied()
        .unwrap_or_default()
}

/// The text of one cell, held inline up to seven bytes.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Glyph([u8; INLINE + 1]);

impl Glyph {
    pub(crate) const fn inline(text: &str) -> Self {
        let source = text.as_bytes();
        assert!(source.len() <= INLINE, "an inline glyph holds seven bytes");
        let mut bytes = [0; INLINE + 1];
        bytes.split_at_mut(source.len()).0.copy_from_slice(source);
        let [.., len] = &mut bytes;
        #[allow(
            clippy::cast_possible_truncation,
            reason = "the assert above bounds the length to seven"
        )]
        {
            *len = source.len() as u8;
        }
        Self(bytes)
    }

    pub(crate) const fn ascii(byte: u8) -> Self {
        Self([byte, 0, 0, 0, 0, 0, 0, 1])
    }

    pub(crate) fn new(text: &str) -> Self {
        if text.len() <= INLINE {
            return Self::inline(text);
        }
        intern(&GLYPHS, text).map_or(Self::inline(REPLACEMENT), |id| {
            let [low, high] = id.to_le_bytes();
            Self([low, high, 0, 0, 0, 0, 0, LONG])
        })
    }

    fn long(self) -> Option<&'static str> {
        let [low, high, .., mark] = self.0;
        (mark == LONG).then(|| resolve(&GLYPHS, u16::from_le_bytes([low, high])))
    }

    fn short(&self) -> &[u8] {
        self.0
            .get(..usize::from(self.0[INLINE]))
            .unwrap_or_default()
    }

    pub(crate) fn as_bytes(&self) -> &[u8] {
        self.long().map_or_else(|| self.short(), str::as_bytes)
    }

    pub(crate) fn as_str(&self) -> &str {
        if let Some(text) = self.long() {
            return text;
        }
        match self.short() {
            [byte] if byte.is_ascii() => {
                let at = usize::from(*byte);
                ASCII.get(at..=at).unwrap_or_default()
            }
            bytes => std::str::from_utf8(bytes).unwrap_or_default(),
        }
    }
}

impl std::ops::Deref for Glyph {
    type Target = str;

    fn deref(&self) -> &str {
        self.as_str()
    }
}

impl std::fmt::Debug for Glyph {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self.as_str(), f)
    }
}

impl From<&str> for Glyph {
    fn from(text: &str) -> Self {
        Self::new(text)
    }
}

impl From<String> for Glyph {
    fn from(text: String) -> Self {
        Self::new(&text)
    }
}

impl PartialEq<str> for Glyph {
    fn eq(&self, other: &str) -> bool {
        self.as_bytes() == other.as_bytes()
    }
}

impl PartialEq<&str> for Glyph {
    fn eq(&self, other: &&str) -> bool {
        self.as_bytes() == other.as_bytes()
    }
}

/// The hyperlink of one cell.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct Link(NonZeroU16);

impl Link {
    pub(crate) fn new(url: &str) -> Option<Self> {
        let id = intern(&LINKS, url)?;
        NonZeroU16::new(id.checked_add(1)?).map(Self)
    }

    pub(crate) fn as_str(self) -> &'static str {
        resolve(&LINKS, self.0.get() - 1)
    }
}

impl std::fmt::Debug for Link {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self.as_str(), f)
    }
}
