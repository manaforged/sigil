use smallvec::SmallVec;
use unicode_bidi::BidiInfo;

#[must_use]
pub fn paragraph_levels(input: &str) -> SmallVec<[u8; 64]> {
    if is_latin_only(input) {
        return SmallVec::from_elem(0, input.len());
    }
    level_numbers(&BidiInfo::new(input, None).levels)
}

fn level_numbers(levels: &[unicode_bidi::Level]) -> SmallVec<[u8; 64]> {
    levels.iter().map(unicode_bidi::Level::number).collect()
}

fn is_latin_only(input: &str) -> bool {
    input.chars().all(|c| c <= '\u{024F}')
}

#[must_use]
pub fn visual_order(levels: &[u8]) -> Vec<usize> {
    if levels.len() <= 1 {
        return (0..levels.len()).collect();
    }
    let bidi_levels: Vec<unicode_bidi::Level> = levels
        .iter()
        .map(|&l| unicode_bidi::Level::new(l).unwrap_or(unicode_bidi::LTR_LEVEL))
        .collect();
    unicode_bidi::BidiInfo::reorder_visual(&bidi_levels)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    To(usize),
    Edge,
    Logical,
}

pub struct ParagraphBidi<'a> {
    text: &'a str,
    info: Option<unicode_bidi::ParagraphBidiInfo<'a>>,
}

impl<'a> ParagraphBidi<'a> {
    #[must_use]
    pub fn new(text: &'a str) -> Self {
        if is_latin_only(text) {
            return Self { text, info: None };
        }
        Self {
            text,
            info: Some(unicode_bidi::ParagraphBidiInfo::new(text, None)),
        }
    }

    #[must_use]
    pub fn visual_step(&self, caret: usize, right: bool) -> Step {
        let Some(ltr) = self.uniform_ltr() else {
            return Step::Logical;
        };
        let target = if ltr == right {
            next_char(self.text, caret)
        } else {
            prev_char(self.text, caret)
        };
        target.map_or(Step::Edge, Step::To)
    }

    fn uniform_ltr(&self) -> Option<bool> {
        let Some(info) = &self.info else {
            return Some(true);
        };
        let levels = info.reordered_levels(0..self.text.len());
        let first = *levels.first()?;
        levels
            .iter()
            .all(|level| *level == first)
            .then(|| first.is_ltr())
    }

    #[must_use]
    pub fn levels(&self) -> SmallVec<[u8; 64]> {
        self.info.as_ref().map_or_else(
            || SmallVec::from_elem(0, self.text.len()),
            |info| level_numbers(&info.levels),
        )
    }

    #[must_use]
    pub fn visual_order_for_line(
        &self,
        line_start: usize,
        line_end: usize,
        segment_ends: &[u32],
    ) -> SmallVec<[usize; 16]> {
        let Some(info) = self.info.as_ref().filter(|info| {
            segment_ends.len() > 1 && line_start < line_end && line_end <= info.levels.len()
        }) else {
            return (0..segment_ends.len()).collect();
        };
        let line_levels = info.reordered_levels(line_start..line_end);
        let levels: Vec<u8> = segment_ends
            .iter()
            .map(|&end| {
                let last_byte = (end as usize).saturating_sub(1);
                line_levels
                    .get(last_byte)
                    .map_or(0, unicode_bidi::Level::number)
            })
            .collect();
        SmallVec::from_vec(visual_order(&levels))
    }
}

fn next_char(text: &str, i: usize) -> Option<usize> {
    if i >= text.len() {
        return None;
    }
    text.get(i..)?
        .char_indices()
        .nth(1)
        .map_or(Some(text.len()), |(offset, _)| Some(i + offset))
}

fn prev_char(text: &str, i: usize) -> Option<usize> {
    if i == 0 {
        return None;
    }
    text.get(..i)?.char_indices().next_back().map(|(o, _)| o)
}

#[cfg(test)]
#[path = "bidi/step_tests.rs"]
mod step_tests;

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "test/instrumentation context; the invariant is proven by the caller"
)]
#[path = "bidi/paragraph_tests.rs"]
mod tests;
