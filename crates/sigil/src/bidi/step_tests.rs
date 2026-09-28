use super::*;

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
    assert_eq!(
        to(ParagraphBidi::new("abc").visual_step(1, true)),
        Some(2),
        "Right = forward"
    );
    assert_eq!(
        to(ParagraphBidi::new("abc").visual_step(1, false)),
        Some(0),
        "Left = backward"
    );
    assert_eq!(
        to(ParagraphBidi::new("abc").visual_step(0, false)),
        None,
        "Left edge"
    );
    assert_eq!(
        to(ParagraphBidi::new("abc").visual_step(3, true)),
        None,
        "Right edge"
    );
}

#[test]
fn pure_rtl_reverses() {
    assert_eq!(
        to(ParagraphBidi::new(HE).visual_step(0, false)),
        Some(2),
        "RTL Left = forward"
    );
    assert_eq!(
        to(ParagraphBidi::new(HE).visual_step(0, true)),
        None,
        "RTL visual-right edge"
    );
    assert_eq!(
        to(ParagraphBidi::new(HE).visual_step(6, true)),
        Some(4),
        "RTL Right = backward"
    );
    assert_eq!(
        to(ParagraphBidi::new(HE).visual_step(6, false)),
        None,
        "RTL visual-left edge"
    );
}

#[test]
fn mixed_line_defers_to_logical() {
    assert_eq!(
        ParagraphBidi::new("aאבc").visual_step(1, true),
        Step::Logical
    );
}
