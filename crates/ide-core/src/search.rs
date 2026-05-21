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
//! use ide_core::search::{fuzzy, fuzzy_with, Match};
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
mod tests {
    #![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

    use super::*;

    #[test]
    fn empty_needle_returns_input_order() {
        let hay = ["alpha", "beta", "gamma"];
        let out = fuzzy("", &hay);
        assert_eq!(out.len(), 3);
        assert_eq!(out[0].idx, 0);
        assert_eq!(out[1].idx, 1);
        assert_eq!(out[2].idx, 2);
        for m in &out {
            assert!(m.positions.is_empty());
            assert_eq!(m.score, 0);
        }
    }

    #[test]
    fn empty_needle_respects_limit() {
        let hay = ["a", "b", "c", "d"];
        let out = fuzzy_with("", &hay, 2);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].idx, 0);
        assert_eq!(out[1].idx, 1);
    }

    #[test]
    fn zero_limit_returns_empty() {
        let hay = ["alpha"];
        assert!(fuzzy_with("a", &hay, 0).is_empty());
        assert!(fuzzy_with("", &hay, 0).is_empty());
    }

    #[test]
    fn single_char_needle_does_not_panic() {
        let hay = ["", "a", "ab", "no-match-here"];
        let _ = fuzzy("a", &hay);
        let _ = fuzzy("z", &hay);
        let _ = fuzzy("/", &hay);
    }

    #[test]
    fn exact_beats_prefix_beats_substring() {
        let hay = ["main", "main.rs", "src/main.rs", "remaining"];
        let out = fuzzy("main", &hay);
        // Best match must be the exact one.
        assert_eq!(out[0].idx, 0);
        // Prefix `main.rs` must outrank embedded `remaining`.
        let main_rs = out.iter().position(|m| m.idx == 1).unwrap();
        let remaining = out.iter().position(|m| m.idx == 3).unwrap();
        assert!(
            main_rs < remaining,
            "prefix `main.rs` should outrank substring `remaining`: out = {out:?}"
        );
    }

    #[test]
    fn case_sensitive_outranks_case_insensitive() {
        let hay = ["Foo", "foo"];
        let out = fuzzy("foo", &hay);
        assert_eq!(out[0].idx, 1, "case-sensitive exact `foo` wins over `Foo`");
    }

    #[test]
    fn camelcase_boundary_boost() {
        // Needle "cO" exercises smart-case: uppercase 'O' opts into
        // case-sensitive prefix, so "coca" cannot win on PREFIX_ICASE
        // alone. The subsequence path then awards `createObject` the
        // camel-boundary bonus on the 'O' hit.
        let hay = ["coca", "createObject"];
        let out = fuzzy("cO", &hay);
        assert_eq!(out[0].idx, 1);
    }

    #[test]
    fn path_segment_boost() {
        // "ml" should prefer "src/main/lib.rs" (m + l both at
        // segment heads) over "amalgam-light".
        let hay = ["amalgam-light", "src/main/lib.rs"];
        let out = fuzzy("ml", &hay);
        assert_eq!(out[0].idx, 1);
    }

    #[test]
    fn non_ascii_handled() {
        let hay = ["café-menu", "naïve.txt", "résumé/cover.md", "plain.txt"];
        // Should not panic and should match plain ASCII subsequences.
        let out = fuzzy("cm", &hay);
        assert!(!out.is_empty());
        // Non-ascii needle is also fine.
        let out2 = fuzzy("é", &hay);
        assert!(!out2.is_empty());
    }

    #[test]
    fn no_match_returns_empty() {
        let hay = ["alpha", "beta", "gamma"];
        let out = fuzzy("zzz", &hay);
        assert!(out.is_empty());
    }

    #[test]
    fn limit_truncates_after_sort() {
        let hay = ["main", "main.rs", "src/main.rs", "manifest", "remaining"];
        let out = fuzzy_with("main", &hay, 2);
        assert_eq!(out.len(), 2);
        // Best two must include the exact match.
        assert!(out.iter().any(|m| m.idx == 0));
    }

    #[test]
    fn positions_are_ascending_and_in_bounds() {
        let hay = ["src/main.rs"];
        let out = fuzzy("smr", &hay);
        assert_eq!(out.len(), 1);
        let m = &out[0];
        let len = hay[0].len();
        let mut last = 0usize;
        for (i, &p) in m.positions.iter().enumerate() {
            assert!(p < len);
            if i > 0 {
                assert!(p > last);
            }
            last = p;
        }
    }

    #[test]
    fn monotone_dense_beats_sparse() {
        // Two haystacks, identical alphabet. The first has the needle
        // chars consecutive (densest possible); the second spreads
        // them out. Dense must score >= sparse.
        let dense = "abc-something-else";
        let sparse = "a-x-b-x-c-something-else";
        let hay = [dense, sparse];
        let out = fuzzy("abc", &hay);
        assert_eq!(out.len(), 2);
        let dense_score = out.iter().find(|m| m.idx == 0).unwrap().score;
        let sparse_score = out.iter().find(|m| m.idx == 1).unwrap().score;
        assert!(
            dense_score >= sparse_score,
            "dense {dense_score} should be >= sparse {sparse_score}"
        );
    }

    #[test]
    fn deterministic_tiebreak_by_idx() {
        let hay = ["same", "same", "same"];
        let out = fuzzy("same", &hay);
        assert_eq!(out.len(), 3);
        // Scores all equal; order must be idx ascending.
        assert_eq!(out[0].idx, 0);
        assert_eq!(out[1].idx, 1);
        assert_eq!(out[2].idx, 2);
    }
}
