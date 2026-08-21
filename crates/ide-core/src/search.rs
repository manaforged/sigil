//! Search — owned fuzzy-match for command palette + file picker + future
//! search consumers (I-T5).
//!
//! The algorithm is a Smith-Waterman-flavoured matcher with bonuses for
//! camelCase, `snake_case` and path-segment boundaries. Single-pass
//! greedy with a tight inner loop — no `nucleo`, no `fuzzy-matcher`,
//! no external string-distance crates.
//!
//! # API
//!
//! ```ignore
//! use sigil_ide_core::search::{fuzzy, fuzzy_with, Match};
//!
//! let hay = ["src/main.rs", "src/lib.rs", "docs/readme.md"];
//! let hits: Vec<Match> = fuzzy("mn", &hay);
//! // hits sorted by score descending; `hits[0].idx` is the best.
//!
//! let top3 = fuzzy_with("rs", &hay, 3);
//! ```
//!
//! # Ranking summary
//!
//! Best-to-worst, given needle `n` and haystack entry `h`:
//!
//! 1. `h == n` (case-sensitive exact)
//! 2. `h.eq_ignore_ascii_case(n)` (case-insensitive exact)
//! 3. `h.starts_with(n)` (prefix)
//! 4. word-boundary-anchored subsequence (camelCase / `_` / `/` / `-`)
//! 5. dense subsequence (consecutive characters)
//! 6. sparse subsequence (large gaps)
//!
//! Case-sensitive ranks above case-insensitive at every tier.

use smallvec::SmallVec;

mod fuzzy;
mod score;

/// Maximum positions tracked inline before spilling to heap.
///
/// 16 covers typical needle lengths (file names, command palette
/// queries). Larger needles are correct but allocate.
pub const INLINE_POSITIONS: usize = 16;

/// One match result. `idx` indexes back into the haystack slice
/// passed to [`fuzzy`] / [`fuzzy_with`].
#[derive(Debug, Clone)]
pub struct Match {
    /// Original index into the haystack.
    pub idx: usize,
    /// Score; higher is better. Always `>= 0` for a returned match.
    pub score: i32,
    /// Byte indices in the haystack entry where the needle's
    /// characters matched, ascending. Empty when needle is empty.
    pub positions: SmallVec<[usize; INLINE_POSITIONS]>,
}

/// Run a fuzzy match over `haystack`, returning every entry that
/// matched, sorted by score descending. Ties resolve by `idx`
/// ascending so the order is deterministic.
///
/// An empty `needle` returns every entry in input order with score
/// zero and empty positions.
#[must_use]
pub fn fuzzy(needle: &str, haystack: &[&str]) -> Vec<Match> {
    fuzzy_with(needle, haystack, usize::MAX)
}

/// Like [`fuzzy`] but caps the returned vector at `limit` entries.
/// `limit == 0` returns an empty vector.
#[must_use]
pub fn fuzzy_with(needle: &str, haystack: &[&str], limit: usize) -> Vec<Match> {
    if limit == 0 {
        return Vec::new();
    }
    if needle.is_empty() {
        return empty_needle(haystack, limit);
    }
    let mut out: Vec<Match> = Vec::with_capacity(haystack.len().min(limit.saturating_mul(2)));
    for (idx, hay) in haystack.iter().enumerate() {
        if let Some((score, positions)) = fuzzy::score_entry(needle, hay) {
            out.push(Match {
                idx,
                score,
                positions,
            });
        }
    }
    out.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.idx.cmp(&b.idx)));
    if out.len() > limit {
        out.truncate(limit);
    }
    out
}

fn empty_needle(haystack: &[&str], limit: usize) -> Vec<Match> {
    let take = haystack.len().min(limit);
    (0..take)
        .map(|idx| Match {
            idx,
            score: 0,
            positions: SmallVec::new(),
        })
        .collect()
}

#[cfg(test)]
#[path = "search_tests.rs"]
mod tests;
