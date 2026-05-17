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
mod tests {
    use super::*;

    #[test]
    fn graphemes_empty_yields_nothing() {
        assert_eq!(graphemes("").count(), 0);
    }

    #[test]
    fn words_ascii_pair() {
        let collected: Vec<&str> = words("hello world").collect();
        assert_eq!(collected, vec!["hello", "world"]);
    }

    #[test]
    fn graphemes_single_emoji_is_one_cluster() {
        // Single dart-board emoji (U+1F3AF) is 4 bytes UTF-8, one cluster.
        let dart: Vec<&str> = graphemes("\u{1F3AF}").collect();
        assert_eq!(dart.len(), 1);
        assert_eq!(dart[0], "\u{1F3AF}");
        // Thumbs-up (U+1F44D) also one cluster.
        let thumbs: Vec<&str> = graphemes("\u{1F44D}").collect();
        assert_eq!(thumbs, vec!["\u{1F44D}"]);
    }

    #[test]
    fn graphemes_zwj_family_is_one_cluster() {
        // Man + ZWJ + Woman + ZWJ + Girl — one extended grapheme cluster.
        let family = "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}";
        let collected: Vec<&str> = graphemes(family).collect();
        assert_eq!(collected.len(), 1, "ZWJ family must be one cluster");
        assert_eq!(collected[0], family);
    }

    #[test]
    fn graphemes_combining_acute_is_one_cluster() {
        // 'e' + COMBINING ACUTE ACCENT (U+0301) renders as one é cluster.
        let composed = "e\u{0301}";
        let collected: Vec<&str> = graphemes(composed).collect();
        assert_eq!(collected.len(), 1);
        assert_eq!(collected[0], composed);
    }

    #[test]
    fn sentences_two_sentences() {
        // split_sentence_bounds yields every span (incl. whitespace gaps).
        // Filter to those ending with a period to count "sentences".
        let count = sentences("Foo. Bar.")
            .filter(|s| s.trim_end().ends_with('.'))
            .count();
        assert_eq!(count, 2);
    }

    #[test]
    fn words_respect_punctuation() {
        // Punctuation is stripped from word-like spans.
        let collected: Vec<&str> = words("foo, bar; baz!").collect();
        assert_eq!(collected, vec!["foo", "bar", "baz"]);
    }

    #[test]
    fn graphemes_mixed_cjk_latin() {
        let collected: Vec<&str> = graphemes("Hello \u{4E16}\u{754C}").collect();
        // H,e,l,l,o,space,世,界 = 8 clusters.
        assert_eq!(collected.len(), 8);
        assert_eq!(collected[6], "\u{4E16}");
        assert_eq!(collected[7], "\u{754C}");
    }

    #[test]
    fn graphemes_byte_offsets_round_trip() {
        // Reconstruct byte offsets by walking the source, verifying every
        // yielded segment is exactly `&input[offset..offset+segment.len()]`.
        let input = "He\u{0301}llo \u{1F3AF} \u{4E16}";
        let mut offset = 0;
        for seg in graphemes(input) {
            let end = offset + seg.len();
            assert_eq!(&input[offset..end], seg, "offset {offset} mismatch");
            offset = end;
        }
        assert_eq!(offset, input.len(), "offsets must cover all bytes");
    }

    #[test]
    fn graphemes_crlf_is_one_cluster() {
        // CR + LF is a single extended grapheme cluster per UAX #29.
        let collected: Vec<&str> = graphemes("a\r\nb").collect();
        assert_eq!(collected, vec!["a", "\r\n", "b"]);
    }

    #[test]
    fn words_cjk_split_per_character() {
        // unicode_words() splits CJK by ideograph; punctuation is dropped.
        let collected: Vec<&str> = words("\u{4E16}\u{754C}, hello").collect();
        assert_eq!(collected, vec!["\u{4E16}", "\u{754C}", "hello"]);
    }
}
