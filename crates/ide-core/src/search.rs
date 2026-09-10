use smallvec::SmallVec;

mod fuzzy;
mod score;

pub const INLINE_POSITIONS: usize = 16;

#[derive(Debug, Clone)]
pub struct Match {
    pub idx: usize,
    pub score: i32,
    pub positions: SmallVec<[usize; INLINE_POSITIONS]>,
}

#[must_use]
pub fn fuzzy(needle: &str, haystack: &[&str]) -> Vec<Match> {
    fuzzy_with(needle, haystack, usize::MAX)
}

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
