//! Unicode segmentation wrappers — graphemes, words, sentences.
//!
//! Thin newtype wrappers over `unicode-segmentation`. The G1.2 body
//! delegates straight to the upstream iterators; the wrappers exist
//! so downstream crates can name the iterator type without pulling
//! in `unicode-segmentation` directly (DR-044 lib boundary).

use unicode_segmentation::{UnicodeSegmentation, UnicodeWords};

/// Iterator over extended grapheme clusters in a string.
pub struct GraphemeIter<'a> {
    inner: unicode_segmentation::Graphemes<'a>,
}

impl<'a> Iterator for GraphemeIter<'a> {
    type Item = &'a str;
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

/// Iterator over Unicode words (UAX #29 word boundaries, filtered to
/// word-like spans).
pub struct WordIter<'a> {
    inner: UnicodeWords<'a>,
}

impl<'a> Iterator for WordIter<'a> {
    type Item = &'a str;
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

/// Iterator over Unicode sentences (UAX #29 sentence boundaries).
pub struct SentenceIter<'a> {
    inner: unicode_segmentation::USentenceBounds<'a>,
}

impl<'a> Iterator for SentenceIter<'a> {
    type Item = &'a str;
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

/// Iterate extended grapheme clusters in `input`.
#[must_use]
pub fn graphemes(input: &str) -> GraphemeIter<'_> {
    GraphemeIter {
        inner: input.graphemes(true),
    }
}

/// Iterate Unicode words in `input`.
#[must_use]
pub fn words(input: &str) -> WordIter<'_> {
    WordIter {
        inner: input.unicode_words(),
    }
}

/// Iterate Unicode sentences in `input`.
#[must_use]
pub fn sentences(input: &str) -> SentenceIter<'_> {
    SentenceIter {
        inner: input.split_sentence_bounds(),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
#[path = "segment_tests.rs"]
mod tests;
