//! Empty-line byte-position tests (adversarial review follow-up, finding
//! B). Consecutive hard breaks produce empty [`LayoutLine`]s; before
//! this fix they all shared a `u32::MAX..0` sentinel byte range, so
//! hit-testing any of them placed the caret at the document start
//! (byte 0) instead of at the break that created that specific line.

#![allow(missing_docs, clippy::unwrap_used, clippy::indexing_slicing)]

use sigil_hal::{HitPosition, ParagraphMetrics, TextAffinity, TextEngine};
use sigil_text::{OwnedTextEngine, TextConfig};

fn engine() -> OwnedTextEngine {
    OwnedTextEngine::new(TextConfig::default())
}

#[test]
fn middle_empty_line_hit_test_returns_the_break_byte_not_zero() {
    // "a\n\nb": line0="a" (byte 0..1), line1=EMPTY (the blank line
    // between the two breaks — must sit at byte 2, right after the
    // first \n), line2="b" (byte 3..4).
    let e = engine();
    let text = "a\n\nb";
    let p = e.layout_paragraph(text, None).unwrap();
    assert_eq!(
        p.line_count(),
        3,
        "two consecutive \\n must produce 3 lines"
    );

    // Hit-test somewhere on line1's row (y between one and two line
    // heights down) — must resolve to byte 2, not byte 0.
    let line_height = p.height() / 3.0;
    let middle_line_y = line_height * 1.5;
    let hit = p.hit_test(0.0, middle_line_y);
    assert_eq!(
        hit.byte_index, 2,
        "hit-testing the middle empty line must return byte 2 (right after \
         the first \\n), not the document start",
    );
}

#[test]
fn caret_rect_for_the_break_byte_lands_on_the_empty_line_not_line_zero() {
    let e = engine();
    let text = "a\n\nb";
    let p = e.layout_paragraph(text, None).unwrap();
    let line_height = p.height() / 3.0;

    let pos = HitPosition {
        byte_index: 2,
        affinity: TextAffinity::Downstream,
    };
    let rect = p.caret_rect(pos, 1.0);

    // line1's top edge is exactly one line_height down from line0's.
    assert!(
        rect.origin.y >= line_height - 0.5 && rect.origin.y < 2.0 * line_height,
        "caret_rect(byte 2) must land on line 1 (y in [{}, {})), got y={}",
        line_height,
        2.0 * line_height,
        rect.origin.y,
    );
}

#[test]
fn three_consecutive_hard_breaks_give_each_empty_line_a_distinct_byte() {
    // "a\n\n\nb": TWO empty lines back to back (line1 at byte 2, line2
    // at byte 3) — they must not collapse onto the same position.
    let e = engine();
    let text = "a\n\n\nb";
    let p = e.layout_paragraph(text, None).unwrap();
    assert_eq!(p.line_count(), 4);

    let line_height = p.height() / 4.0;
    let hit_line1 = p.hit_test(0.0, line_height * 1.5);
    let hit_line2 = p.hit_test(0.0, line_height * 2.5);
    assert_eq!(hit_line1.byte_index, 2, "first empty line must be byte 2");
    assert_eq!(hit_line2.byte_index, 3, "second empty line must be byte 3");
    assert_ne!(
        hit_line1.byte_index, hit_line2.byte_index,
        "two consecutive empty lines must not collapse onto the same byte",
    );
}

#[test]
fn trailing_hard_break_empty_line_hit_tests_to_its_own_end_byte() {
    // "a\n": one hard break, producing a trailing EMPTY final line at
    // byte 2 (right after the \n, at the text's own end).
    let e = engine();
    let text = "a\n";
    let p = e.layout_paragraph(text, None).unwrap();
    assert_eq!(
        p.line_count(),
        2,
        "trailing \\n must produce an empty final line"
    );

    let line_height = p.height() / 2.0;
    let hit = p.hit_test(0.0, line_height * 1.5);
    assert_eq!(
        hit.byte_index,
        text.len(),
        "the trailing empty line must hit-test to the text's own end byte",
    );
}
