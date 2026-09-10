#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ByteIdx(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CharIdx(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LineCol {
    pub line: usize,
    pub col: usize,
}

impl ByteIdx {
    #[must_use]
    pub const fn new(byte: usize) -> Self {
        Self(byte)
    }
}

impl CharIdx {
    #[must_use]
    pub const fn new(char_idx: usize) -> Self {
        Self(char_idx)
    }
}

impl LineCol {
    #[must_use]
    pub const fn new(line: usize, col: usize) -> Self {
        Self { line, col }
    }
}

#[must_use]
pub fn count_chars(bytes: &[u8]) -> usize {
    bytes.iter().filter(|&&b| is_char_start(b)).count()
}

#[must_use]
pub fn count_lines(bytes: &[u8]) -> usize {
    count_newlines(bytes)
}

#[inline]
#[must_use]
const fn is_char_start(b: u8) -> bool {
    (b & 0b1100_0000) != 0b1000_0000
}

#[must_use]
pub fn count_utf16(bytes: &[u8]) -> usize {
    let mut units = 0usize;
    for &b in bytes {
        if is_char_start(b) {
            units += 1;
            if b >= 0xF0 {
                units += 1;
            }
        }
    }
    units
}

#[allow(clippy::naive_bytecount)]
fn count_newlines(bytes: &[u8]) -> usize {
    bytes.iter().filter(|&&b| b == b'\n').count()
}

#[must_use]
pub fn byte_to_char_in_leaf(bytes: &[u8], byte: usize) -> usize {
    debug_assert!(
        byte <= bytes.len(),
        "byte_to_char_in_leaf: byte out of bounds"
    );
    let prefix = bytes.get(..byte).unwrap_or(&[]);
    count_chars(prefix)
}

#[must_use]
pub fn char_to_byte_in_leaf(bytes: &[u8], char_idx: usize) -> usize {
    let mut seen = 0usize;
    for (i, &b) in bytes.iter().enumerate() {
        if is_char_start(b) {
            if seen == char_idx {
                return i;
            }
            seen += 1;
        }
    }
    bytes.len()
}

#[must_use]
pub fn utf16_to_byte_in_leaf(bytes: &[u8], utf16: usize) -> usize {
    let mut seen = 0usize;
    for (i, &b) in bytes.iter().enumerate() {
        if is_char_start(b) {
            if seen >= utf16 {
                return i;
            }
            seen += if b >= 0xF0 { 2 } else { 1 };
        }
    }
    bytes.len()
}

#[must_use]
pub fn byte_to_utf16_in_leaf(bytes: &[u8], byte: usize) -> usize {
    debug_assert!(
        byte <= bytes.len(),
        "byte_to_utf16_in_leaf: byte out of bounds"
    );
    let prefix = bytes.get(..byte).unwrap_or(&[]);
    count_utf16(prefix)
}

#[must_use]
pub fn line_start_in_leaf(bytes: &[u8], line_idx: usize) -> usize {
    if line_idx == 0 {
        return 0;
    }
    let mut seen = 0usize;
    for (i, &b) in bytes.iter().enumerate() {
        if b == b'\n' {
            seen += 1;
            if seen == line_idx {
                return i + 1;
            }
        }
    }
    bytes.len()
}

#[must_use]
pub fn lines_before_byte_in_leaf(bytes: &[u8], byte: usize) -> usize {
    let prefix = bytes.get(..byte).unwrap_or(&[]);
    count_lines(prefix)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]
#[path = "coords_tests.rs"]
mod tests;
