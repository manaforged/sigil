//! Coordinate types for the rope buffer.
//!
//! A rope addresses text under three useful coordinate systems:
//!
//! - [`ByteIdx`] — UTF-8 byte offset (the canonical storage coordinate).
//! - [`CharIdx`] — Unicode scalar value count.
//! - [`LineCol`] — line + UTF-8 byte column inside the line.
//!
//! The rope also exposes UTF-16 code-unit offsets (`byte_to_utf16`,
//! `utf16_to_byte`) for LSP protocol interop, but the offset itself is
//! a plain `usize` — there's no `Utf16Idx` newtype because no other
//! call site needs to type-check it separately.

/// UTF-8 byte offset into a rope (0..=`len_bytes`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ByteIdx(pub usize);

/// Unicode scalar value (char) offset into a rope (0..=`len_chars`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CharIdx(pub usize);

/// Line + UTF-8 byte column within the line.
///
/// `line` is 0-based; `col` is the UTF-8 byte offset from the start of
/// the line, NOT a char count. Column-as-bytes matches the storage
/// coordinate; convert to char-or-grapheme columns at the display
/// layer if you need visual columns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LineCol {
    /// 0-based line index.
    pub line: usize,
    /// UTF-8 byte offset within the line.
    pub col: usize,
}

impl ByteIdx {
    /// Construct a byte index.
    #[must_use]
    pub const fn new(byte: usize) -> Self {
        Self(byte)
    }
}

impl CharIdx {
    /// Construct a char index.
    #[must_use]
    pub const fn new(char_idx: usize) -> Self {
        Self(char_idx)
    }
}

impl LineCol {
    /// Construct a line + byte-column.
    #[must_use]
    pub const fn new(line: usize, col: usize) -> Self {
        Self { line, col }
    }
}

/// Count Unicode scalar values in a UTF-8 slice.
#[must_use]
pub fn count_chars(bytes: &[u8]) -> usize {
    // Counting char boundaries (bytes whose top 2 bits are not `10`) is
    // equivalent to counting `str::chars()` and ~3× faster on large
    // inputs because it skips UTF-8 decoding.
    bytes.iter().filter(|&&b| is_char_start(b)).count()
}

/// Count `\n` bytes in a UTF-8 slice.
#[must_use]
pub fn count_lines(bytes: &[u8]) -> usize {
    count_newlines(bytes)
}

/// Returns `true` for any UTF-8 byte that begins a code point
/// (i.e. is not a continuation byte). Continuation bytes match the bit
/// pattern `10xx_xxxx`; everything else starts a code point.
#[inline]
#[must_use]
const fn is_char_start(b: u8) -> bool {
    (b & 0b1100_0000) != 0b1000_0000
}

/// Count UTF-16 code units a UTF-8 slice would encode to.
///
/// One BMP code point is 1 UTF-16 unit; one supplementary code point
/// (U+10000..) is 2 UTF-16 units. The UTF-8 byte that starts a 4-byte
/// sequence (`0xF0..=0xF4`) is exactly the supplementary case, so we
/// add one extra unit per such byte on top of the char count.
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

/// Plain byte-counting newline counter. The rope is zero-runtime-deps
/// by policy (DR-040), so we don't pull in `bytecount` or `memchr` —
/// the naive count is fast enough for the 512-byte target leaf size.
#[allow(clippy::naive_bytecount)]
fn count_newlines(bytes: &[u8]) -> usize {
    bytes.iter().filter(|&&b| b == b'\n').count()
}

/// Convert a byte offset within a UTF-8 leaf into a char offset.
///
/// Returns the number of UTF-8 chars that start before `byte`. The
/// caller must guarantee `byte <= bytes.len()` and `byte` is a char
/// boundary.
#[must_use]
pub fn byte_to_char_in_leaf(bytes: &[u8], byte: usize) -> usize {
    debug_assert!(
        byte <= bytes.len(),
        "byte_to_char_in_leaf: byte out of bounds"
    );
    let prefix = bytes.get(..byte).unwrap_or(&[]);
    count_chars(prefix)
}

/// Convert a char offset within a UTF-8 leaf into a byte offset.
///
/// Walks the UTF-8 bytes counting char starts until `char_idx` chars
/// have been passed. Returns `bytes.len()` if `char_idx` exceeds the
/// char count of `bytes`.
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

/// Convert a UTF-16 unit offset within a leaf to a byte offset.
///
/// Returns `bytes.len()` if `utf16` exceeds the leaf's UTF-16 length.
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

/// Convert a byte offset within a leaf to its UTF-16 unit offset.
#[must_use]
pub fn byte_to_utf16_in_leaf(bytes: &[u8], byte: usize) -> usize {
    debug_assert!(
        byte <= bytes.len(),
        "byte_to_utf16_in_leaf: byte out of bounds"
    );
    let prefix = bytes.get(..byte).unwrap_or(&[]);
    count_utf16(prefix)
}

/// Find the byte position immediately after the `line_idx`-th `\n`.
///
/// `line_idx == 0` returns 0 (start of the leaf). Returns `bytes.len()`
/// if the leaf has fewer than `line_idx` newlines.
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

/// Count `\n` bytes strictly before `byte`.
#[must_use]
pub fn lines_before_byte_in_leaf(bytes: &[u8], byte: usize) -> usize {
    let prefix = bytes.get(..byte).unwrap_or(&[]);
    count_lines(prefix)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn counts_match_str_chars_lines_utf16() {
        let s = "hello\nworld\n🦀abc";
        let b = s.as_bytes();
        assert_eq!(count_chars(b), s.chars().count());
        assert_eq!(count_lines(b), s.bytes().filter(|&c| c == b'\n').count());
        assert_eq!(count_utf16(b), s.encode_utf16().count());
    }

    #[test]
    fn byte_char_roundtrips_in_leaf() {
        let s = "α🦀β\nγ";
        let b = s.as_bytes();
        for (ci, (bi, _)) in s.char_indices().enumerate() {
            assert_eq!(byte_to_char_in_leaf(b, bi), ci);
            assert_eq!(char_to_byte_in_leaf(b, ci), bi);
        }
    }

    #[test]
    fn line_starts_in_leaf() {
        let s = "a\nbb\n\nccc";
        let b = s.as_bytes();
        assert_eq!(line_start_in_leaf(b, 0), 0);
        assert_eq!(line_start_in_leaf(b, 1), 2);
        assert_eq!(line_start_in_leaf(b, 2), 5);
        assert_eq!(line_start_in_leaf(b, 3), 6);
        assert_eq!(line_start_in_leaf(b, 99), b.len());
    }

    #[test]
    fn utf16_units_match_encode_utf16() {
        let s = "🦀a🦀b";
        let b = s.as_bytes();
        let total = count_utf16(b);
        assert_eq!(total, s.encode_utf16().count());
        let pos = byte_to_utf16_in_leaf(b, 4);
        assert!(pos > 0);
    }
}
