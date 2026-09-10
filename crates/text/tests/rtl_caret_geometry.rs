#![allow(missing_docs, clippy::unwrap_used, clippy::indexing_slicing)]

use sigil_hal::{HitPosition, ParagraphMetrics, TextAffinity, TextEngine};
use sigil_text::{OwnedTextEngine, TextConfig};

const RTL_WORD: &str = "\u{05E9}\u{05DC}\u{05D5}\u{05DD}";

fn engine() -> OwnedTextEngine {
    OwnedTextEngine::new(TextConfig::default())
}

fn caret_x(p: &sigil_text::Paragraph, byte: usize, affinity: TextAffinity) -> f32 {
    p.caret_rect(
        HitPosition {
            byte_index: byte,
            affinity,
        },
        1.0,
    )
    .origin
    .x
}

#[test]
fn pure_rtl_logical_start_caret_sits_at_the_visual_right_edge() {
    let e = engine();
    let p = e.layout_paragraph(RTL_WORD, None).unwrap();
    let width = p.width();
    let start = caret_x(&p, 0, TextAffinity::Downstream);
    assert!(
        start > width * 0.5,
        "RTL logical-start caret must sit in the right half (visual reading \
         start); got {start} of width {width}",
    );
    assert!(
        (start - width).abs() < 2.0,
        "RTL logical-start caret should hug the right edge; got {start} vs {width}",
    );
}

#[test]
fn caret_advances_leftward_through_a_pure_rtl_word() {
    let e = engine();
    let p = e.layout_paragraph(RTL_WORD, None).unwrap();
    let x0 = caret_x(&p, 0, TextAffinity::Downstream);
    let x2 = caret_x(&p, 2, TextAffinity::Downstream);
    let x4 = caret_x(&p, 4, TextAffinity::Downstream);
    assert!(
        x0 > x2 && x2 > x4,
        "caret must march leftward as the logical offset grows in RTL text; \
         got x0={x0} x2={x2} x4={x4}",
    );
}

#[test]
fn affinity_splits_the_ltr_rtl_boundary_into_two_visual_carets() {
    let e = engine();
    let text = "abc \u{05D0}\u{05D1}\u{05D2} def";
    let p = e.layout_paragraph(text, None).unwrap();
    let boundary = 4;
    let upstream = caret_x(&p, boundary, TextAffinity::Upstream);
    let downstream = caret_x(&p, boundary, TextAffinity::Downstream);
    assert!(
        (upstream - downstream).abs() > 1.0,
        "affinity must resolve the LTR/RTL boundary to two distinct visual \
         carets; got upstream={upstream} downstream={downstream}",
    );
}

#[test]
fn selection_across_a_mixed_line_emits_one_rect_per_visual_run() {
    let e = engine();
    let text = "abc \u{05D0}\u{05D1}\u{05D2} def";
    let p = e.layout_paragraph(text, None).unwrap();
    let full = text.len();
    let rects = p.selection_rects(
        HitPosition {
            byte_index: 0,
            affinity: TextAffinity::Downstream,
        },
        HitPosition {
            byte_index: full,
            affinity: TextAffinity::Upstream,
        },
    );
    assert!(!rects.is_empty(), "a non-empty selection must paint");
    let total: f32 = rects.iter().map(|r| r.width).sum();
    assert!(
        total <= p.width() + 1.0,
        "selection rects must not exceed the line width; total={total} width={}",
        p.width(),
    );
    for r in &rects {
        assert!(
            r.origin.x >= -0.5 && r.origin.x + r.width <= p.width() + 0.5,
            "selection rect {:?} escapes the line [0,{}]",
            r,
            p.width(),
        );
    }
}
