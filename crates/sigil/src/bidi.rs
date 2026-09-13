use unicode_bidi::BidiInfo;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    To(usize),
    Edge,
    Logical,
}

#[must_use]
pub fn visual_step(text: &str, caret: usize, right: bool) -> Step {
    let info = BidiInfo::new(text, None);
    let Some(para) = info
        .paragraphs
        .iter()
        .find(|p| p.range.start <= caret && caret <= p.range.end)
    else {
        return Step::Logical;
    };
    let (levels, runs) = info.visual_runs(para, para.range.clone());
    let [run] = runs.as_slice() else {
        return Step::Logical;
    };
    let ltr = levels
        .get(run.start)
        .is_some_and(unicode_bidi::Level::is_ltr);
    let forward = ltr == right;
    let target = if forward {
        next_char(text, caret)
    } else {
        prev_char(text, caret)
    };
    target.map_or(Step::Edge, Step::To)
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
#[path = "bidi/lib_tests.rs"]
mod tests;
