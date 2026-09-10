pub(super) const EXACT: i32 = 10_000;
pub(super) const EXACT_ICASE: i32 = 8_000;
pub(super) const PREFIX: i32 = 6_000;
pub(super) const PREFIX_ICASE: i32 = 5_000;

const BASE: i32 = 10;
const PATH_SEGMENT_BONUS: i32 = 40;
const WORD_BOUNDARY_BONUS: i32 = 30;
const CAMEL_BONUS: i32 = 45;
const SEQUENTIAL_BONUS: i32 = 15;
const CASE_MATCH_BONUS: i32 = 5;
const START_BONUS: i32 = 8;
const GAP_COST: i32 = 3;
pub(super) const LEADING_SKIP_COST: i32 = 1;
pub(super) const LEADING_SKIP_CAP: usize = 20;
const GAP_MAX: usize = 16;

pub(super) struct CharContext<'a> {
    pub byte_idx: usize,
    pub ch: char,
    pub target: char,
    pub prev_match: Option<usize>,
    pub prev_byte: Option<u8>,
    pub hay_bytes: &'a [u8],
}

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

fn utf8_len(bytes: &[u8], idx: usize) -> usize {
    let Some(&b) = bytes.get(idx) else {
        return 1;
    };
    match b {
        0..=0xBF => 1,
        0xC0..=0xDF => 2,
        0xE0..=0xEF => 3,
        _ => 4,
    }
}

pub(super) fn clamp_usize_to_i32(n: usize) -> i32 {
    i32::try_from(n).unwrap_or(i32::MAX)
}

#[cfg(test)]
#[path = "score_tests.rs"]
mod tests;
