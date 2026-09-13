use unicode_segmentation::{UnicodeSegmentation, UnicodeWords};

pub struct GraphemeIter<'a> {
    inner: unicode_segmentation::Graphemes<'a>,
}

impl<'a> Iterator for GraphemeIter<'a> {
    type Item = &'a str;
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

pub struct WordIter<'a> {
    inner: UnicodeWords<'a>,
}

impl<'a> Iterator for WordIter<'a> {
    type Item = &'a str;
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

pub struct SentenceIter<'a> {
    inner: unicode_segmentation::USentenceBounds<'a>,
}

impl<'a> Iterator for SentenceIter<'a> {
    type Item = &'a str;
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

#[must_use]
pub fn graphemes(input: &str) -> GraphemeIter<'_> {
    GraphemeIter {
        inner: input.graphemes(true),
    }
}

#[must_use]
pub fn words(input: &str) -> WordIter<'_> {
    WordIter {
        inner: input.unicode_words(),
    }
}

#[must_use]
pub fn sentences(input: &str) -> SentenceIter<'_> {
    SentenceIter {
        inner: input.split_sentence_bounds(),
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "test/instrumentation context; the invariant is proven by the caller"
)]
#[path = "segment_tests.rs"]
mod tests;
