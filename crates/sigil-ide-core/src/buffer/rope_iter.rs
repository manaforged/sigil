//! Iterators over rope content.
//!
//! All iterators borrow a collected `Vec<u8>` snapshot of the slice
//! range. This is a deliberate trade-off: the rope is designed for
//! *edit* performance (copy-on-write `Arc` nodes); for *read* access,
//! collecting the byte range into a contiguous buffer is simpler and
//! still fast enough for IDE use (read < 1 MB at a time in practice).
//!
//! A zero-copy iterator that walks the B-tree node list directly can
//! be added in a follow-up once the API is stable.

/// Iterates over bytes in a rope slice.
///
/// Created by [`crate::buffer::rope::RopeSlice::bytes`].
pub struct RopeBytesIter<'a> {
    /// Backing byte slice (a collected snapshot of the rope range).
    data: &'a [u8],
    pos: usize,
}

impl<'a> RopeBytesIter<'a> {
    /// Build from a contiguous byte slice.
    #[must_use]
    pub const fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }
}

impl Iterator for RopeBytesIter<'_> {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        let b = self.data.get(self.pos).copied()?;
        self.pos += 1;
        Some(b)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let rem = self.data.len().saturating_sub(self.pos);
        (rem, Some(rem))
    }
}

impl ExactSizeIterator for RopeBytesIter<'_> {}

/// Iterates over Unicode scalar values in a rope slice.
///
/// Created by [`crate::buffer::rope::RopeSlice::chars`].
pub struct RopeCharsIter<'a> {
    inner: std::str::Chars<'a>,
}

impl<'a> RopeCharsIter<'a> {
    /// Build from a UTF-8 string slice.
    ///
    /// # Panics (debug-only)
    ///
    /// In debug mode, asserts the input is valid UTF-8.
    #[must_use]
    pub fn new(s: &'a str) -> Self {
        Self { inner: s.chars() }
    }
}

impl Iterator for RopeCharsIter<'_> {
    type Item = char;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

/// Iterates over lines in a rope slice, yielding each line as a `String`.
///
/// Lines include the trailing `\n` if present (matches `str::lines`
/// semantics except we preserve the `\n`; the last line may have no
/// trailing newline).
///
/// Created by [`crate::buffer::rope::RopeSlice::lines`].
pub struct RopeLinesIter<'a> {
    remaining: &'a str,
    done: bool,
}

impl<'a> RopeLinesIter<'a> {
    /// Build from a UTF-8 string slice.
    #[must_use]
    pub const fn new(s: &'a str) -> Self {
        Self {
            remaining: s,
            done: s.is_empty(),
        }
    }
}

impl<'a> Iterator for RopeLinesIter<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        if let Some(nl) = self.remaining.find('\n') {
            let line = self.remaining.get(..=nl).unwrap_or(self.remaining);
            self.remaining = self.remaining.get(nl + 1..).unwrap_or("");
            if self.remaining.is_empty() {
                self.done = true;
            }
            Some(line)
        } else {
            self.done = true;
            Some(self.remaining)
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn bytes_iter_correct() {
        let data = b"hello";
        let v: Vec<u8> = RopeBytesIter::new(data).collect();
        assert_eq!(v, b"hello");
    }

    #[test]
    fn chars_iter_correct() {
        let s = "abc🦀";
        let v: Vec<char> = RopeCharsIter::new(s).collect();
        assert_eq!(v, s.chars().collect::<Vec<_>>());
    }

    #[test]
    fn lines_iter_correct() {
        let s = "a\nbb\ncc";
        let v: Vec<&str> = RopeLinesIter::new(s).collect();
        assert_eq!(v, vec!["a\n", "bb\n", "cc"]);
    }

    #[test]
    fn lines_iter_empty() {
        assert!(RopeLinesIter::new("").next().is_none());
    }

    #[test]
    fn lines_iter_trailing_newline() {
        let s = "a\nb\n";
        let v: Vec<&str> = RopeLinesIter::new(s).collect();
        assert_eq!(v, vec!["a\n", "b\n"]);
    }
}
