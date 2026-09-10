#![allow(missing_docs, clippy::unwrap_used, clippy::indexing_slicing)]

use sigil_hal::{ParagraphMetrics, TextEngine};
use sigil_text::{OwnedTextEngine, TextConfig};

const WORD_A: &str = "\u{05E9}\u{05DC}\u{05D5}\u{05DD}";
const WORD_B: &str = "\u{05E2}\u{05D5}\u{05DC}\u{05DD}";

fn engine() -> OwnedTextEngine {
    OwnedTextEngine::new(TextConfig::default())
}

fn caret_x(p: &sigil_text::Paragraph, byte_index: usize, affinity: sigil_hal::TextAffinity) -> f32 {
    p.caret_rect(
        sigil_hal::HitPosition {
            byte_index,
            affinity,
        },
        1.0,
    )
    .origin
    .x
}

#[test]
fn embedded_rtl_run_sits_between_ltr_neighbors_and_advances_rtl_internally() {
    let e = engine();
    let text = "abc \u{05D0}\u{05D1}\u{05D2} def";
    let p = e.layout_paragraph(text, None).unwrap();
    assert!(!p.rendered_glyphs().is_empty());

    let hebrew_start = 4_usize;
    let hebrew_end = 10_usize;

    let abc_end_x = caret_x(&p, 3, sigil_hal::TextAffinity::Upstream);
    let hebrew_start_x = caret_x(&p, hebrew_start, sigil_hal::TextAffinity::Downstream);
    let hebrew_end_x = caret_x(&p, hebrew_end, sigil_hal::TextAffinity::Upstream);
    let def_start_x = caret_x(&p, hebrew_end + 1, sigil_hal::TextAffinity::Downstream);

    assert!(
        abc_end_x <= hebrew_start_x.min(hebrew_end_x),
        "the Hebrew run must sit at/after 'abc' ends: abc_end_x={abc_end_x} \
         hebrew=[{hebrew_start_x},{hebrew_end_x}]",
    );
    assert!(
        hebrew_start_x.max(hebrew_end_x) <= def_start_x,
        "the Hebrew run must sit at/before 'def' starts: hebrew=[{hebrew_start_x},{hebrew_end_x}] \
         def_start_x={def_start_x}",
    );
}

#[test]
fn pure_rtl_multiword_paragraph_reverses_visual_word_order() {
    let e = engine();
    let text = format!("{WORD_A} {WORD_B}");
    let p = e.layout_paragraph(&text, None).unwrap();

    let word_b_start = u32::try_from(WORD_A.len() + 1).unwrap();
    let leftmost_hit = p.hit_test(0.0, 0.0);
    assert!(
        leftmost_hit.byte_index >= word_b_start as usize,
        "leftmost x must land in word B (visually first in an RTL paragraph), \
         got byte_index {} (word B starts at {word_b_start})",
        leftmost_hit.byte_index,
    );

    let word_a_end = u32::try_from(WORD_A.len()).unwrap();
    let rightmost_hit = p.hit_test(p.width() - 1.0, 0.0);
    assert!(
        rightmost_hit.byte_index <= word_a_end as usize,
        "rightmost x must land in word A (visually last in an RTL paragraph), \
         got byte_index {} (word A ends at {word_a_end})",
        rightmost_hit.byte_index,
    );

    let caret_leftmost = p.caret_rect(leftmost_hit, 1.0);
    let caret_rightmost = p.caret_rect(rightmost_hit, 1.0);
    assert!(
        caret_leftmost.origin.x < caret_rightmost.origin.x,
        "leftmost-hit caret ({}) must sit left of rightmost-hit caret ({}) on the \
         reordered line",
        caret_leftmost.origin.x,
        caret_rightmost.origin.x,
    );
    assert!(
        caret_leftmost.origin.x <= 1.0,
        "leftmost hit's caret should sit near the line's left edge, got {}",
        caret_leftmost.origin.x,
    );
}

#[test]
fn wrapped_line_applies_l1_reset_not_just_paragraph_wide_levels() {
    let e = engine();
    let text = "\u{05D0}\u{05D1}\u{05D2} abc def";
    let p = e.layout_paragraph(text, Some(70.0)).unwrap();
    assert_eq!(
        p.line_count(),
        2,
        "expected width=70 to wrap 'def' onto its own line, got {} lines",
        p.line_count(),
    );

    let abc_start = caret_x(&p, 7, sigil_hal::TextAffinity::Downstream);
    assert!(
        abc_start > 0.5,
        "L1-corrected line must push 'abc' off the line's left edge (the \
         trailing space, reset to paragraph level, must lead the visual \
         order instead) — got abc start x={abc_start}",
    );
}

#[test]
fn three_plus_wrapped_mixed_direction_lines_with_trailing_spaces_do_not_panic() {
    let e = engine();
    let text = format!("{WORD_A} abc {WORD_B} def {WORD_A} ghi");
    let p = e.layout_paragraph(&text, Some(45.0)).unwrap();
    assert!(
        p.line_count() >= 3,
        "expected width=45 to wrap into 3+ lines, got {}",
        p.line_count(),
    );

    for line in 0..p.line_count() {
        #[allow(clippy::cast_precision_loss)]
        let y = (line as f32).mul_add(20.0, 5.0);
        for frac in [0.0_f32, 0.5, 1.0] {
            let pos = p.hit_test(p.width() * frac, y);
            assert!(
                text.is_char_boundary(pos.byte_index),
                "hit_test byte_index {} is not a UTF-8 char boundary at line {line}, frac {frac}",
                pos.byte_index,
            );
            let rect = p.caret_rect(pos, 1.0);
            assert!(
                rect.origin.x.is_finite() && rect.origin.x >= 0.0,
                "caret x must be finite and non-negative at line {line}, frac {frac}, got {}",
                rect.origin.x,
            );
        }
    }
}
