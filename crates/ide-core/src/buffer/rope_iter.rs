pub struct RopeBytesIter<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> RopeBytesIter<'a> {
    #[must_use]
    pub const fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }
}

impl Iterator for RopeBytesIter<'_> {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        let b = self.data.get(self.pos).copied()?;
        self.pos += 1;
        Some(b)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let rem = self.data.len().saturating_sub(self.pos);
        (rem, Some(rem))
    }
}

impl ExactSizeIterator for RopeBytesIter<'_> {}

pub struct RopeCharsIter<'a> {
    inner: std::str::Chars<'a>,
}

impl<'a> RopeCharsIter<'a> {
    #[must_use]
    pub fn new(s: &'a str) -> Self {
        Self { inner: s.chars() }
    }
}

impl Iterator for RopeCharsIter<'_> {
    type Item = char;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

pub struct RopeLinesIter<'a> {
    remaining: &'a str,
    done: bool,
}

impl<'a> RopeLinesIter<'a> {
    #[must_use]
    pub const fn new(s: &'a str) -> Self {
        Self {
            remaining: s,
            done: s.is_empty(),
        }
    }
}

impl<'a> Iterator for RopeLinesIter<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        if let Some(nl) = self.remaining.find('\n') {
            let line = self.remaining.get(..=nl).unwrap_or(self.remaining);
            self.remaining = self.remaining.get(nl + 1..).unwrap_or("");
            if self.remaining.is_empty() {
                self.done = true;
            }
            Some(line)
        } else {
            self.done = true;
            Some(self.remaining)
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]
#[path = "rope_iter_tests.rs"]
mod tests;
