pub(crate) struct NextMatch {
    needle: String,
    ignore_case: bool,
    tail: Option<usize>,
    hit: Option<usize>,
}

impl NextMatch {
    pub(crate) fn new(needle: &str, ignore_case: bool) -> Self {
        Self {
            needle: needle.to_string(),
            ignore_case,
            tail: None,
            hit: None,
        }
    }

    pub(crate) fn find(&mut self, rest: &str) -> Option<usize> {
        let len = rest.len();
        let cached =
            self.tail.is_some_and(|tail| len <= tail) && self.hit.is_none_or(|hit| hit <= len);
        if !cached {
            let found = if self.ignore_case {
                find_ci(rest, &self.needle)
            } else {
                rest.find(self.needle.as_str())
            };
            self.tail = Some(len);
            self.hit = found.map(|at| len.saturating_sub(at));
        }
        self.hit.map(|hit| len.saturating_sub(hit))
    }
}

#[derive(Default)]
pub(crate) struct CloseTags {
    tags: Vec<NextMatch>,
}

impl CloseTags {
    pub(crate) fn find(&mut self, rest: &str, needle: &str) -> Option<usize> {
        if !self.tags.iter().any(|tag| tag.needle == needle) {
            self.tags.push(NextMatch::new(needle, true));
        }
        self.tags
            .iter_mut()
            .find(|tag| tag.needle == needle)?
            .find(rest)
    }
}

pub(crate) fn starts_ci(rest: &str, prefix: &str) -> bool {
    rest.as_bytes()
        .get(..prefix.len())
        .is_some_and(|head| head.eq_ignore_ascii_case(prefix.as_bytes()))
}

pub(crate) fn find_ci(hay: &str, needle: &str) -> Option<usize> {
    let needle = needle.as_bytes();
    hay.as_bytes()
        .windows(needle.len())
        .position(|window| window.eq_ignore_ascii_case(needle))
}
