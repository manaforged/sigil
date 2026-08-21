//! Scoring constants + per-char bonus table.
//!
//! These constants are tuned so that the ranking tiers in the
//! `search` module doc-comment hold: exact > prefix > word-boundary
//! subsequence > dense subsequence > sparse subsequence.

/// Whole-string exact, case-sensitive.
pub(super) const EXACT: i32 = 10_000;
/// Whole-string exact, case-insensitive.
pub(super) const EXACT_ICASE: i32 = 8_000;
/// Prefix, case-sensitive.
pub(super) const PREFIX: i32 = 6_000;
/// Prefix, case-insensitive.
pub(super) const PREFIX_ICASE: i32 = 5_000;

/// Base reward per matched character.
const BASE: i32 = 10;
/// Bonus when the matched byte sits at a path-segment head
/// (immediately after `/` or `\`).
const PATH_SEGMENT_BONUS: i32 = 40;
/// Bonus at a generic word boundary (`_`, `-`, `.`, ` `).
const WORD_BOUNDARY_BONUS: i32 = 30;
/// Bonus at a camelCase boundary (uppercase preceded by lowercase
/// in the haystack). Tuned high enough that a camel hit at a moderate
/// gap (≤ 8 bytes) outranks a fully sequential lowercase match — see
/// `camelcase_boundary_boost` in `super::tests`.
const CAMEL_BONUS: i32 = 45;
/// Bonus when the match is contiguous with the previous one.
const SEQUENTIAL_BONUS: i32 = 15;
/// Bonus when the haystack char matches the needle char with
/// identical case.
const CASE_MATCH_BONUS: i32 = 5;
/// Bonus when the match is the very first byte of the haystack.
const START_BONUS: i32 = 8;
/// Penalty per skipped char between this match and the prior one.
const GAP_COST: i32 = 3;
/// Penalty per byte skipped before the first match (capped).
pub(super) const LEADING_SKIP_COST: i32 = 1;
/// Cap on the leading-skip penalty so a single deep match doesn't
/// wipe the score.
pub(super) const LEADING_SKIP_CAP: usize = 20;
/// Cap on the per-gap penalty so a single gap can't tip the whole
/// score below zero.
const GAP_MAX: usize = 16;

/// Inputs for [`per_char_bonus`]. Bundled so we stay under the
/// 5-arg rust-hygiene cap. Borrowed by reference so the call is
/// allocation-free.
pub(super) struct CharContext<'a> {
    pub byte_idx: usize,
    pub ch: char,
    pub target: char,
    pub prev_match: Option<usize>,
    pub prev_byte: Option<u8>,
    pub hay_bytes: &'a [u8],
}

/// Reward for a single matched char given its context. Always
/// returns `BASE + sum-of-bonuses` (non-negative on its own; the
/// gap penalty subtracts).
pub(super) fn per_char_bonus(ctx: &CharContext<'_>) -> i32 {
    let mut bonus = BASE;
    if ctx.ch == ctx.target {
        bonus += CASE_MATCH_BONUS;
    }
    if ctx.byte_idx == 0 {
        bonus += START_BONUS;
    }
    if is_path_segment_head(ctx.byte_idx, ctx.prev_byte) {
        bonus += PATH_SEGMENT_BONUS;
    } else if is_word_boundary(ctx.byte_idx, ctx.prev_byte) {
        bonus += WORD_BOUNDARY_BONUS;
    } else if is_camel_boundary(ctx.ch, ctx.prev_byte) {
        bonus += CAMEL_BONUS;
    }
    if let Some(prev) = ctx.prev_match {
        let gap = ctx
            .byte_idx
            .saturating_sub(prev)
            .saturating_sub(utf8_len(ctx.hay_bytes, prev));
        if gap == 0 {
            bonus += SEQUENTIAL_BONUS;
        } else {
            bonus -= clamp_usize_to_i32(gap.min(GAP_MAX)) * GAP_COST;
        }
    }
    bonus
}

const fn is_path_segment_head(byte_idx: usize, prev_byte: Option<u8>) -> bool {
    if byte_idx == 0 {
        return false;
    }
    matches!(prev_byte, Some(b'/' | b'\\'))
}

const fn is_word_boundary(byte_idx: usize, prev_byte: Option<u8>) -> bool {
    if byte_idx == 0 {
        return false;
    }
    matches!(prev_byte, Some(b'_' | b'-' | b'.' | b' '))
}

const fn is_camel_boundary(ch: char, prev_byte: Option<u8>) -> bool {
    if !ch.is_ascii_uppercase() {
        return false;
    }
    matches!(prev_byte, Some(p) if p.is_ascii_lowercase())
}

/// Byte-width of the UTF-8 code-point starting at `bytes[idx]`.
/// Returns 1 for ASCII / continuation / out-of-bounds so a stale
/// `prev_match` cannot crash.
fn utf8_len(bytes: &[u8], idx: usize) -> usize {
    let Some(&b) = bytes.get(idx) else {
        return 1;
    };
    match b {
        0..=0xBF => 1, // ASCII + continuation bytes (length irrelevant for our use).
        0xC0..=0xDF => 2,
        0xE0..=0xEF => 3,
        _ => 4,
    }
}

/// `usize → i32` saturating clamp. We only ever feed in values
/// bounded by [`GAP_MAX`] or [`LEADING_SKIP_CAP`], so the cast is
/// always safe — clamp belt-and-braces against future refactors.
pub(super) fn clamp_usize_to_i32(n: usize) -> i32 {
    i32::try_from(n).unwrap_or(i32::MAX)
}

#[cfg(test)]
#[path = "score_tests.rs"]
mod tests;
