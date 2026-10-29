use super::super::fuzzy::fuzzy_score;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Matcher {
    Substring,
    Prefix,
    Fuzzy,
}

impl Matcher {
    pub(crate) fn score(self, query: &str, label: &str) -> Option<usize> {
        match self {
            Self::Substring => {
                let query = query.to_lowercase();
                (query.is_empty() || label.to_lowercase().contains(&query)).then_some(0)
            }
            Self::Prefix => label
                .to_lowercase()
                .starts_with(&query.to_lowercase())
                .then_some(0),
            Self::Fuzzy => fuzzy_score(query, label).map(|found| found.score),
        }
    }

    pub(crate) fn rank<'a>(
        self,
        query: &str,
        labels: impl IntoIterator<Item = &'a str>,
    ) -> Vec<usize> {
        let mut hits: Vec<(usize, usize)> = labels
            .into_iter()
            .enumerate()
            .filter_map(|(index, label)| self.score(query, label).map(|score| (score, index)))
            .collect();
        hits.sort_by_key(|(score, _)| *score);
        hits.into_iter().map(|(_, index)| index).collect()
    }
}
