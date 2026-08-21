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
