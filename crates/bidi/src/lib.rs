//! Visual-order caret stepping for bidirectional text.
//!
//! Logical caret movement (previous/next char) is wrong for RTL text: in
//! an RTL run, pressing Left should move the caret *visually* left, which
//! is *logically forward*. [`visual_step`] resolves a line into bidi runs
//! and, for a single-direction run, steps the caret by its visual
//! neighbour.
//!
//! Scope: a line that is a single bidi run (pure LTR *or* pure RTL) is
//! handled exactly. A mixed-direction line returns [`Step::Logical`] so the
//! caller keeps ordinary logical movement — correct visual navigation
//! across a direction boundary needs an affinity-aware caret (a byte offset
//! can map to two visual positions there), which a byte-offset caret model
//! does not carry.
//!
//! This lives in its own crate so the `unicode-bidi` dependency stays off
//! the always-on widget facade (which has dozens of test targets that would
//! otherwise each need to acknowledge an unused dependency).

#![deny(missing_docs)]

use unicode_bidi::BidiInfo;

/// Result of a one-position visual caret step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// Move the caret to this byte offset.
    To(usize),
    /// Single-direction line, but already at its visual edge: stay put.
    Edge,
    /// Mixed-direction line (or no resolvable paragraph): the caller should
    /// fall back to logical movement.
    Logical,
}

/// Step the caret one visual position (`right` = display-right) within the
/// paragraph of `text` containing `caret` (a byte offset at a char
/// boundary).
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
    // Only a single-direction line is affinity-free.
    let [run] = runs.as_slice() else {
        return Step::Logical;
    };
    // Visual-right in an LTR run is logical-forward; in an RTL run it is
    // logical-backward.
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

/// Byte offset of the next char boundary after `i`, or `None` at the end.
fn next_char(text: &str, i: usize) -> Option<usize> {
    if i >= text.len() {
        return None;
    }
    text.get(i..)?
        .char_indices()
        .nth(1)
        .map_or(Some(text.len()), |(offset, _)| Some(i + offset))
}

/// Byte offset of the char boundary before `i`, or `None` at the start.
fn prev_char(text: &str, i: usize) -> Option<usize> {
    if i == 0 {
        return None;
    }
    text.get(..i)?.char_indices().next_back().map(|(o, _)| o)
}

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;
