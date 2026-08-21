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
#[path = "bidi_tests.rs"]
mod tests;
