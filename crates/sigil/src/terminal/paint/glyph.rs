use std::num::NonZeroU16;
use std::sync::{LazyLock, PoisonError, RwLock};

use ahash::AHashMap;

const INLINE: usize = 7;
const LONG: u8 = u8::MAX;
const ENTRIES: usize = u16::MAX as usize - 1;
const ENTRY_BYTES: usize = 2048;
const REPLACEMENT: &str = "\u{fffd}";
const ASCII_COUNT: usize = 128;

const ASCII: &str = {
    const BYTES: [u8; ASCII_COUNT] = {
        let mut bytes = [0; ASCII_COUNT];
        let mut at = 0;
        while at < ASCII_COUNT {
            bytes[at] = at as u8;
            at += 1;
        }
        bytes
    };
    match std::str::from_utf8(&BYTES) {
        Ok(text) => text,
        Err(_) => "",
    }
};

#[derive(Default)]
struct Table {
    ids: AHashMap<&'static str, u16>,
    texts: Vec<&'static str>,
}

type Shared = LazyLock<RwLock<Table>>;

static GLYPHS: Shared = LazyLock::new(RwLock::default);
static LINKS: Shared = LazyLock::new(RwLock::default);

fn intern(table: &Shared, text: &str) -> Option<u16> {
    let known = |table: &Table| table.ids.get(text).copied();
    if let Some(id) = known(&table.read().unwrap_or_else(PoisonError::into_inner)) {
        return Some(id);
    }
    let mut table = table.write().unwrap_or_else(PoisonError::into_inner);
    if let Some(id) = known(&table) {
        return Some(id);
    }
    if table.texts.len() >= ENTRIES || text.len() > ENTRY_BYTES {
        return None;
    }
    let id = u16::try_from(table.texts.len()).ok()?;
    let kept: &'static str = Box::leak(Box::from(text));
    table.texts.push(kept);
    table.ids.insert(kept, id);
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
        let mut at = 0;
        while at < source.len() {
            bytes[at] = source[at];
            at += 1;
        }
        bytes[INLINE] = source.len() as u8;
        Self(bytes)
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
