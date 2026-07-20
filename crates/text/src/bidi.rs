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

/// Compute the UAX #9 rule L2 visual (paint) order of `levels`.
///
/// Each entry is one already-uniform-level unit (an engine
/// `ShapedSegment`, not necessarily a single codepoint) — L2 reverses
/// contiguous same-or-higher-level runs from the highest embedding
/// level down to the lowest odd level, which is well-defined at either
/// granularity since a uniform-level unit never needs to split mid-run.
/// Delegates to `unicode_bidi::BidiInfo::reorder_visual`, the crate's
/// own L2 implementation, rather than re-deriving it.
///
/// Returns a permutation where `result[visual_position] ==
/// logical_index`; an out-of-range level (defensive only — this
/// crate's levels are always `unicode_bidi`-derived and `<=125`) maps
/// to LTR so a single bad level can't panic the whole line.
#[must_use]
pub fn visual_order(levels: &[u8]) -> Vec<usize> {
    if levels.len() <= 1 {
        return (0..levels.len()).collect();
    }
    let bidi_levels: Vec<unicode_bidi::Level> = levels
        .iter()
        .map(|&l| unicode_bidi::Level::new(l).unwrap_or(unicode_bidi::LTR_LEVEL))
        .collect();
    unicode_bidi::BidiInfo::reorder_visual(&bidi_levels)
}

/// Per-paragraph bidi analysis, computed once and reused for both
/// shaping-run segmentation (paragraph-wide levels, before line
/// breaking is known) and visual line reordering (line-aware levels).
///
/// Recomputing `unicode_bidi`'s class-detection pass per LINE instead
/// of once per PARAGRAPH would cost `O(lines × paragraph_len)` instead
/// of `O(paragraph_len)` — this type exists so the engine can build it
/// once in `build_paragraph` and thread it through both the segmenter
/// and the line-assembly pass.
pub struct ParagraphBidi<'a> {
    len: usize,
    info: Option<unicode_bidi::ParagraphBidiInfo<'a>>,
}

impl<'a> ParagraphBidi<'a> {
    /// Analyse `text` once. `info` is `None` on the Latin-only fast
    /// path (`is_latin_only`) — every level is 0 there, so UAX #9
    /// rules L1/L2 are no-ops and there's nothing to precompute.
    #[must_use]
    pub fn new(text: &'a str) -> Self {
        if text.is_empty() || is_latin_only(text) {
            return Self {
                len: text.len(),
                info: None,
            };
        }
        Self {
            len: text.len(),
            info: Some(unicode_bidi::ParagraphBidiInfo::new(text, None)),
        }
    }

    /// Per-byte UAX #9 embedding levels for the whole paragraph — the
    /// segmenter's run-splitting input. Pre-L1: line boundaries aren't
    /// known yet at segmentation time, and L1 only matters at rule L2
    /// (visual reorder), which runs per finalized line via
    /// [`Self::visual_order_for_line`].
    #[must_use]
    pub fn levels(&self) -> SmallVec<[u8; 64]> {
        let Some(info) = &self.info else {
            let mut out = SmallVec::new();
            out.resize(self.len, 0u8);
            return out;
        };
        let mut out = SmallVec::with_capacity(info.levels.len());
        out.extend(info.levels.iter().map(unicode_bidi::Level::number));
        out
    }

    /// Visual order of `segment_ends` — the paragraph-relative
    /// exclusive end byte of each already-uniform-level segment on one
    /// finalized line, in logical (source) order — over the byte range
    /// `[line_start, line_end)`.
    ///
    /// Applies UAX #9 rule L1 (reset trailing whitespace/separators to
    /// the paragraph embedding level for THIS line — `reordered_levels`
    /// runs L1 internally) before rule L2 ([`visual_order`]), via
    /// `unicode_bidi`'s line-aware API — the gap a paragraph-wide
    /// [`visual_order`] call can't close, because L1's trailing-neutral
    /// reset depends on where a line actually ends after word-wrapping,
    /// not on the paragraph's own end.
    #[must_use]
    pub fn visual_order_for_line(
        &self,
        line_start: usize,
        line_end: usize,
        segment_ends: &[u32],
    ) -> Vec<usize> {
        if segment_ends.len() <= 1 {
            return (0..segment_ends.len()).collect();
        }
        let Some(info) = &self.info else {
            return (0..segment_ends.len()).collect();
        };
        if line_end <= line_start || line_end > info.levels.len() {
            return (0..segment_ends.len()).collect();
        }
        let line_levels = info.reordered_levels(line_start..line_end);
        let levels: Vec<u8> = segment_ends
            .iter()
            .map(|&end| {
                let last_byte = (end as usize).saturating_sub(1);
                line_levels
                    .get(last_byte)
                    .map_or(0, unicode_bidi::Level::number)
            })
            .collect();
        visual_order(&levels)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn line_aware_l1_resets_trailing_neutral_that_paragraph_wide_levels_miss() {
        // "אבג abc def" (RTL Hebrew word, two LTR Latin words) inside a
        // pure-RTL-base paragraph. The space BETWEEN "abc" and "def"
        // has a LATIN neighbour on both sides, so `unicode_bidi`'s
        // ordinary (whole-paragraph) neutral resolution matches it to
        // the Latin run's level (2) — correct IF the line keeps
        // running into "def". But if the line is cut right after that
        // space (as if "def" wrapped away), UAX #9 rule L1 requires
        // resetting it to the PARAGRAPH level (1) instead, since it is
        // now trailing whitespace at the line's own end. Paragraph-wide
        // levels alone (pre-Wave-4-fix behaviour) can't see that the
        // line was cut here — this is the exact gap `visual_order_for_line`
        // exists to close.
        let text = "\u{05D0}\u{05D1}\u{05D2} abc def";
        let space_before_def = 10_usize; // byte offset of the space between "abc" and "def"
        let cut = 11_usize; // right after that space — "def" wraps away

        let paragraph_wide = paragraph_levels(text);
        assert_eq!(
            paragraph_wide[space_before_def], 2,
            "sanity: whole-paragraph resolution matches the space to the \
             Latin run (level 2), got {paragraph_wide:?}",
        );

        let pb = ParagraphBidi::new(text);
        let info = pb
            .info
            .as_ref()
            .expect("mixed-script text takes the full bidi path");
        let line_adjusted = info.reordered_levels(0..cut);
        assert_eq!(
            line_adjusted[space_before_def].number(),
            1,
            "line-aware L1 must reset the line-final space to the paragraph \
             level (1), not the whole-paragraph match-neighbour level (2)",
        );
    }

    #[test]
    fn visual_order_for_line_diverges_from_paragraph_wide_reorder_at_a_wrap_point() {
        // Same scenario as above, exercised through the public
        // `ParagraphBidi` API: reordering the line
        // [hebrew_word, space, "abc", space] with line-aware L1 must
        // produce a DIFFERENT permutation than feeding the raw
        // paragraph-wide levels straight into `visual_order` — proving
        // the line-aware path is not a no-op wrapper.
        let text = "\u{05D0}\u{05D1}\u{05D2} abc def";
        let hebrew_end = 6_u32;
        let space1_end = 7_u32;
        let abc_end = 10_u32;
        let space2_end = 11_u32;
        let segment_ends = [hebrew_end, space1_end, abc_end, space2_end];

        let pb = ParagraphBidi::new(text);
        let line_aware = pb.visual_order_for_line(0, space2_end as usize, &segment_ends);

        let paragraph_wide_levels = paragraph_levels(text);
        let raw_levels: Vec<u8> = segment_ends
            .iter()
            .map(|&end| paragraph_wide_levels[end as usize - 1])
            .collect();
        let paragraph_wide_order = visual_order(&raw_levels);

        assert_ne!(
            line_aware, paragraph_wide_order,
            "line-aware L1 reorder must differ from the paragraph-wide-levels \
             reorder at this wrap point — got {line_aware:?} for both",
        );
        // The line-aware result reflects the L1-corrected levels
        // [1, 1, 2, 1]: only "abc" (index 2) sits at level 2, so L2's
        // "reverse contiguous >= max_level" pass touches just that one
        // element (no-op on a length-1 run), then the whole line
        // reverses at level 1 — the trailing space (index 3) ends up
        // FIRST (visually leftmost), not "abc" (index 2).
        assert_eq!(
            line_aware.first().copied(),
            Some(3),
            "expected the trailing space (logical index 3) to lead the \
             visually-reordered line, got {line_aware:?}",
        );
    }

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
