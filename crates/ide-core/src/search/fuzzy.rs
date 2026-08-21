//! Greedy subsequence matcher with bonus scoring.
//!
//! The matcher walks `haystack` once, advancing through `needle` on
//! each ASCII-case-insensitive char hit. It tracks the running score
//! using [`super::score`]'s bonus table.
//!
//! For exact / prefix matches we short-circuit so they always rank
//! above any subsequence result.

use smallvec::SmallVec;

use super::INLINE_POSITIONS;
use super::score;

/// Compute the best score and matched byte positions for `needle`
/// against `haystack`. Returns `None` when no subsequence match
/// exists at all.
///
/// `needle` is non-empty (caller checks). `haystack` may be empty,
/// in which case we return `None`.
pub(super) fn score_entry(
    needle: &str,
    haystack: &str,
) -> Option<(i32, SmallVec<[usize; INLINE_POSITIONS]>)> {
    if haystack.is_empty() {
        return None;
    }
    if let Some(hit) = exact_or_prefix(needle, haystack) {
        return Some(hit);
    }
    subsequence(needle, haystack)
}

fn exact_or_prefix(
    needle: &str,
    haystack: &str,
) -> Option<(i32, SmallVec<[usize; INLINE_POSITIONS]>)> {
    if haystack == needle {
        return Some((score::EXACT, positions_for_prefix(needle)));
    }
    if haystack.starts_with(needle) {
        return Some((score::PREFIX, positions_for_prefix(needle)));
    }
    // Smart-case: when the needle contains an uppercase letter, treat
    // the query as case-sensitive — do not fall back to icase exact /
    // prefix. The subsequence path will still find matches with the
    // appropriate camel-boundary bonuses.
    let icase_allowed = !needle.chars().any(|c| c.is_ascii_uppercase());
    if icase_allowed {
        if haystack.eq_ignore_ascii_case(needle) {
            return Some((score::EXACT_ICASE, positions_for_prefix(needle)));
        }
        let h_lower_starts = haystack
            .get(..needle.len())
            .is_some_and(|head| head.eq_ignore_ascii_case(needle));
        if h_lower_starts {
            return Some((score::PREFIX_ICASE, positions_for_prefix(needle)));
        }
    }
    None
}

fn positions_for_prefix(needle: &str) -> SmallVec<[usize; INLINE_POSITIONS]> {
    needle.char_indices().map(|(i, _)| i).collect()
}

fn subsequence(needle: &str, haystack: &str) -> Option<(i32, SmallVec<[usize; INLINE_POSITIONS]>)> {
    let needle_chars: SmallVec<[char; INLINE_POSITIONS]> = needle.chars().collect();
    let mut positions: SmallVec<[usize; INLINE_POSITIONS]> = SmallVec::new();
    let mut score_acc: i32 = 0;
    let mut needle_idx: usize = 0;
    let mut prev_match: Option<usize> = None;
    let mut prev_byte: Option<u8> = None;
    let hay_bytes = haystack.as_bytes();

    for (byte_idx, ch) in haystack.char_indices() {
        let Some(&target) = needle_chars.get(needle_idx) else {
            break;
        };
        if !chars_match(ch, target) {
            prev_byte = ascii_byte(ch);
            continue;
        }
        let ctx = score::CharContext {
            byte_idx,
            ch,
            target,
            prev_match,
            prev_byte,
            hay_bytes,
        };
        score_acc += score::per_char_bonus(&ctx);
        positions.push(byte_idx);
        prev_match = Some(byte_idx);
        needle_idx += 1;
        prev_byte = ascii_byte(ch);
    }

    if needle_idx < needle_chars.len() {
        return None;
    }
    if let Some(&first) = positions.first() {
        let capped = first.min(score::LEADING_SKIP_CAP);
        score_acc -= score::clamp_usize_to_i32(capped) * score::LEADING_SKIP_COST;
    }
    if score_acc < 0 {
        score_acc = 0;
    }
    Some((score_acc, positions))
}

const fn chars_match(a: char, b: char) -> bool {
    if a == b {
        return true;
    }
    // `char::to_ascii_lowercase` is const-stable; the `==` between
    // two `char`s is also const, so the whole function stays const.
    let la = a.to_ascii_lowercase();
    let lb = b.to_ascii_lowercase();
    la == lb
}

const fn ascii_byte(ch: char) -> Option<u8> {
    if ch.is_ascii() { Some(ch as u8) } else { None }
}

#[cfg(test)]
#[path = "fuzzy_tests.rs"]
mod tests;
