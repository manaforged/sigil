//! Bidi level computation per UAX #9.
//!
//! Per DR-044: Latin-only paragraphs short-circuit to all-zero levels
//! (~95% of UI text); mixed-direction paragraphs delegate to
//! `unicode-bidi` for the full algorithm.

use smallvec::SmallVec;
use unicode_bidi::BidiInfo;

/// Compute per-byte UAX #9 embedding levels for `input`.
///
/// Returns one `u8` per byte of `input`. Latin / numeric / common
/// neutrals short-circuit to all-zero without invoking the full
/// algorithm; any strongly-RTL or unassigned character triggers the
/// `unicode-bidi` path.
#[must_use]
pub fn paragraph_levels(input: &str) -> SmallVec<[u8; 64]> {
    if input.is_empty() {
        return SmallVec::new();
    }

    if is_latin_only(input) {
        let mut out = SmallVec::new();
        out.resize(input.len(), 0u8);
        return out;
    }

    let info = BidiInfo::new(input, None);
    let mut out = SmallVec::with_capacity(info.levels.len());
    out.extend(info.levels.iter().map(unicode_bidi::Level::number));
    out
}

/// Returns `true` when every char of `input` is safely level-0 without
/// running the full bidi algorithm: ASCII, Latin-1 Supplement, Latin
/// Extended-A/B, plus the common neutrals (line breaks, NBSP).
///
/// Conservative: any char outside this range falls through to the full
/// algorithm even when it would have computed to level 0.
fn is_latin_only(input: &str) -> bool {
    input.chars().all(is_latin_or_neutral)
}

/// One char check for the fast path. Matches:
/// - ASCII (`U+0000..=U+007F`)
/// - Latin-1 Supplement (`U+0080..=U+00FF`)
/// - Latin Extended-A (`U+0100..=U+017F`)
/// - Latin Extended-B (`U+0180..=U+024F`)
///
/// Everything past `U+024F` (IPA Extensions, spacing modifiers,
/// combining marks, Greek, Cyrillic, RTL scripts, CJK, emoji) goes
/// through the full algorithm.
const fn is_latin_or_neutral(c: char) -> bool {
    matches!(c as u32, 0x0000..=0x024F)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn latin_only_all_zero() {
        let input = "hello world";
        let levels = paragraph_levels(input);
        assert_eq!(levels.len(), input.len());
        assert!(levels.iter().all(|&l| l == 0), "Latin must be level 0");
    }

    #[test]
    fn empty_input_empty_levels() {
        let levels = paragraph_levels("");
        assert!(levels.is_empty());
    }

    #[test]
    fn hebrew_only_rtl_levels() {
        // Hebrew letters Alef, Bet, Gimel.
        let input = "\u{05D0}\u{05D1}\u{05D2}";
        let levels = paragraph_levels(input);
        assert_eq!(levels.len(), input.len());
        // Hebrew paragraph base direction is RTL; every byte sits at
        // an odd level (1 for a pure-Hebrew paragraph).
        assert!(
            levels.iter().all(|&l| l % 2 == 1),
            "Hebrew paragraph must be odd-leveled, got {levels:?}",
        );
    }

    #[test]
    fn mixed_latin_hebrew_alternates() {
        // "abc" + Alef + Bet
        let input = "abc\u{05D0}\u{05D1}";
        let levels = paragraph_levels(input);
        assert_eq!(levels.len(), input.len());
        // First three bytes are ASCII -> level 0 (paragraph base is LTR
        // because the first strong character is Latin).
        assert_eq!(&levels[0..3], &[0, 0, 0]);
        // The Hebrew bytes sit at an odd level (>=1).
        for (i, &l) in levels.iter().enumerate().skip(3) {
            assert!(l % 2 == 1, "byte {i} expected odd level, got {l}");
        }
    }

    #[test]
    fn arabic_with_ascii_neutrals() {
        // Arabic Alef + space + ASCII digits + space + Arabic Beh.
        let input = "\u{0627} 12 \u{0628}";
        let levels = paragraph_levels(input);
        assert_eq!(levels.len(), input.len());
        // Paragraph base direction is RTL (first strong char is Arabic).
        // Arabic letter bytes are at the odd RTL level.
        let alef_len = '\u{0627}'.len_utf8();
        for i in 0..alef_len {
            assert!(levels[i] % 2 == 1, "Arabic alef byte {i} must be odd");
        }
        // The ASCII digits get an EN (European Number) level — even
        // (level 2) inside an RTL paragraph per UAX #9.
        let digit_start = alef_len + 1; // skip space
        assert_eq!(
            levels[digit_start], 2,
            "ASCII digit inside RTL paragraph should be level 2",
        );
    }

    #[test]
    fn all_numeric_is_zero() {
        let input = "123 456";
        let levels = paragraph_levels(input);
        assert_eq!(levels.len(), input.len());
        assert!(
            levels.iter().all(|&l| l == 0),
            "ASCII numerics must be level 0 (LTR paragraph)",
        );
    }

    #[test]
    fn lrm_mark_does_not_panic() {
        // U+200E LEFT-TO-RIGHT MARK — explicit bidi formatting char.
        // Must route through the full algorithm without crashing.
        let input = "ab\u{200E}cd";
        let levels = paragraph_levels(input);
        assert_eq!(levels.len(), input.len());
        // ASCII bytes stay at level 0.
        assert_eq!(levels[0], 0);
        assert_eq!(levels[1], 0);
    }

    #[test]
    fn latin_extended_stays_fast_path() {
        // 'é' as U+00E9 (NFC) is Latin-1 Supplement — fast path.
        let input = "caf\u{00E9}";
        let levels = paragraph_levels(input);
        assert_eq!(levels.len(), input.len());
        assert!(levels.iter().all(|&l| l == 0));
    }
}
