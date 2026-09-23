const EXACT: usize = 0;
const PREFIX: usize = 10;
const BOUNDARY: usize = 20;
const SUBSTRING: usize = 30;
const GAPPED: usize = 40;
const MAX_SPREAD: usize = 3;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FuzzyMatch {
    pub score: usize,
    pub positions: Vec<usize>,
}

#[must_use]
pub fn fuzzy_score(query: &str, text: &str) -> Option<FuzzyMatch> {
    let query: Vec<char> = query.to_lowercase().chars().collect();
    let text: Vec<char> = text.to_lowercase().chars().collect();
    if query.is_empty() {
        return Some(FuzzyMatch {
            score: EXACT,
            positions: Vec::new(),
        });
    }
    contiguous(&query, &text).or_else(|| gapped(&query, &text))
}

fn contiguous(query: &[char], text: &[char]) -> Option<FuzzyMatch> {
    let starts: Vec<usize> = text
        .windows(query.len())
        .enumerate()
        .filter(|(_, window)| *window == query)
        .map(|(at, _)| at)
        .collect();
    let first = *starts.first()?;
    let boundary = starts
        .iter()
        .find(|at| **at == 0 || text.get(**at - 1).is_some_and(|ch| is_boundary(*ch)));
    let (score, start) = match boundary {
        Some(0) if text.len() == query.len() => (EXACT, 0),
        Some(0) => (PREFIX, 0),
        Some(at) => (BOUNDARY, *at),
        None => (SUBSTRING, first),
    };
    Some(FuzzyMatch {
        score,
        positions: (start..start + query.len()).collect(),
    })
}

fn gapped(query: &[char], text: &[char]) -> Option<FuzzyMatch> {
    let (first, rest) = query.split_first()?;
    let positions = (0..text.len())
        .filter(|at| text.get(*at) == Some(first))
        .filter_map(|start| {
            let mut positions = vec![start];
            let mut at = start;
            for want in rest {
                at += 1 + text.get(at + 1..)?.iter().position(|ch| ch == want)?;
                positions.push(at);
            }
            Some(positions)
        })
        .min_by_key(|positions| spread(positions))?;
    let spread = spread(&positions);
    (spread <= query.len() * MAX_SPREAD).then(|| FuzzyMatch {
        score: GAPPED + spread - query.len(),
        positions,
    })
}

fn spread(positions: &[usize]) -> usize {
    match (positions.first(), positions.last()) {
        (Some(first), Some(last)) => last + 1 - first,
        _ => 0,
    }
}

const fn is_boundary(ch: char) -> bool {
    ch.is_ascii_whitespace() || matches!(ch, '/' | '-' | '_' | '.' | ':')
}
