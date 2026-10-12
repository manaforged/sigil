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
fn single_run_steps_follow_direction() {
    let cases = [
        ("abc", 1, true, Some(2)),
        ("abc", 1, false, Some(0)),
        ("abc", 0, false, None),
        ("abc", 3, true, None),
        (HE, 0, false, Some(2)),
        (HE, 0, true, None),
        (HE, 6, true, Some(4)),
        (HE, 6, false, None),
    ];
    for (text, caret, right, want) in cases {
        assert_eq!(
            to(ParagraphBidi::new(text).visual_step(caret, right)),
            want,
            "{text:?} caret {caret} right {right}"
        );
    }
}

#[test]
fn mixed_line_defers_to_logical() {
    assert_eq!(
        ParagraphBidi::new("aאבc").visual_step(1, true),
        Step::Logical
    );
}
