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
mod tests {
    use super::*;

    // Hebrew "אבג" — 3 letters, 2 bytes each; logical offsets 0,2,4,6.
    // Displayed right-to-left, so the visual-right edge is logical 0.
    const HE: &str = "אבג";

    fn to(step: Step) -> Option<usize> {
        match step {
            Step::To(p) => Some(p),
            Step::Edge => None,
            Step::Logical => unreachable!("expected a single-run step"),
        }
    }

    #[test]
    fn pure_ltr_moves_logically() {
        assert_eq!(to(visual_step("abc", 1, true)), Some(2), "Right = forward");
        assert_eq!(to(visual_step("abc", 1, false)), Some(0), "Left = backward");
        assert_eq!(to(visual_step("abc", 0, false)), None, "Left edge");
        assert_eq!(to(visual_step("abc", 3, true)), None, "Right edge");
    }

    #[test]
    fn pure_rtl_reverses() {
        // Caret at logical start (0) is the visual-RIGHT edge: Left moves
        // logically forward (visually left); Right has nowhere to go.
        assert_eq!(to(visual_step(HE, 0, false)), Some(2), "RTL Left = forward");
        assert_eq!(to(visual_step(HE, 0, true)), None, "RTL visual-right edge");
        // Logical end (6) is the visual-LEFT edge.
        assert_eq!(
            to(visual_step(HE, 6, true)),
            Some(4),
            "RTL Right = backward"
        );
        assert_eq!(to(visual_step(HE, 6, false)), None, "RTL visual-left edge");
    }

    #[test]
    fn mixed_line_defers_to_logical() {
        // "aאבc" has an LTR run, an RTL run, and an LTR run — visual
        // navigation there needs affinity, so we defer to logical.
        assert_eq!(visual_step("aאבc", 1, true), Step::Logical);
    }
}
