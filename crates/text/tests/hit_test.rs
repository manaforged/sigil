//! Hit-testing, caret geometry, and selection geometry tests.

#![allow(missing_docs, clippy::unwrap_used, clippy::indexing_slicing)]

use ahash as _;
use criterion as _;
#[cfg(feature = "shaping-complex")]
use harfrust as _;
use kernel as _;
use layout as _;
use reactive as _;
use skrifa as _;
use smallvec as _;
use style as _;
use tracing as _;
use unicode_bidi as _;
use unicode_script as _;
use unicode_segmentation as _;

use hal::{HitPosition, ParagraphMetrics, TextAffinity, TextEngine};
use text::{OwnedTextEngine, TextConfig};

fn engine() -> OwnedTextEngine {
    OwnedTextEngine::new(TextConfig::default())
}

// ---------------------------------------------------------------------------
// hit_test
// ---------------------------------------------------------------------------

#[test]
fn hit_test_samples_monotonically_increase_with_x() {
    // As x advances left→right, the returned byte_index must be non-decreasing.
    // This proves hit_test actually traverses the text in order, not just
    // returning 0 for all positions.
    let e = engine();
    let text = "ABCDE";
    let para = e.layout_paragraph(text, None).unwrap();
    let hits: Vec<_> = [0.0_f32, 15.0, 30.0, 45.0, 60.0, 99_999.0]
        .iter()
        .map(|&x| para.hit_test(x, 0.0).byte_index)
        .collect();
    for w in hits.windows(2) {
        assert!(
            w[0] <= w[1],
            "byte_index must be non-decreasing as x increases: {hits:?}"
        );
    }
    assert_eq!(*hits.last().unwrap(), text.len());
}

#[test]
fn hit_test_empty_string_does_not_panic() {
    // An empty text field must not crash on a hit-test at origin.
    let e = engine();
    let para = e.layout_paragraph("", None).unwrap();
    let pos = para.hit_test(0.0, 0.0);
    assert_eq!(pos.byte_index, 0, "empty string hit should return byte 0");
}

#[test]
fn hit_test_far_right_returns_end_of_text() {
    let e = engine();
    let text = "Hello";
    let para = e.layout_paragraph(text, None).unwrap();
    let pos = para.hit_test(99_999.0, 0.0);
    assert_eq!(
        pos.byte_index,
        text.len(),
        "far-right hit should snap to text end"
    );
}

#[test]
fn hit_test_middle_lands_between_endpoints() {
    let e = engine();
    let text = "Hello world";
    let para = e.layout_paragraph(text, None).unwrap();

    let start = para.hit_test(0.0, 0.0);
    let end = para.hit_test(99_999.0, 0.0);
    let mid = para.hit_test(para.width() / 2.0, 0.0);

    assert!(
        mid.byte_index > start.byte_index,
        "mid ({}) should be past start ({})",
        mid.byte_index,
        start.byte_index
    );
    assert!(
        mid.byte_index < end.byte_index,
        "mid ({}) should be before end ({})",
        mid.byte_index,
        end.byte_index
    );
}

#[test]
fn hit_test_multibyte_utf8_index_is_char_boundary() {
    // Every byte_index returned by hit_test must be a valid UTF-8 boundary.
    // "café🔥end" mixes 1-byte ASCII, 2-byte accented char, and 4-byte emoji.
    let e = engine();
    let text = "café🔥end";
    let para = e.layout_paragraph(text, None).unwrap();
    let width = para.width();
    for frac in [0.0_f32, 0.2, 0.4, 0.6, 0.8, 1.0] {
        let pos = para.hit_test(width * frac, 0.0);
        assert!(
            text.is_char_boundary(pos.byte_index),
            "byte_index {} is not a UTF-8 char boundary in {:?}",
            pos.byte_index,
            text
        );
    }
}

#[test]
fn hit_right_half_of_interior_multibyte_lands_after_it() {
    // Regression: clicking the right half of an interior multibyte glyph must
    // place the caret AFTER it, not snap a whole codepoint to the left.
    // "aébc": a=0, é=1..3 (2 bytes), b=3, c=4. The post-é boundary is byte 3.
    let e = engine();
    let text = "aébc";
    let para = e.layout_paragraph(text, None).unwrap();
    // Locate é's pixel span via caret geometry, then hit its right quarter.
    let x_start = para
        .caret_rect(
            HitPosition {
                byte_index: 1,
                affinity: TextAffinity::Downstream,
            },
            1.0,
        )
        .origin
        .x;
    let x_end = para
        .caret_rect(
            HitPosition {
                byte_index: 3,
                affinity: TextAffinity::Upstream,
            },
            1.0,
        )
        .origin
        .x;
    let right_of_e = x_start + (x_end - x_start) * 0.75;

    let hit = para.hit_test(right_of_e, 0.0);
    assert!(
        text.is_char_boundary(hit.byte_index),
        "hit byte {} is mid-codepoint in {text:?}",
        hit.byte_index
    );
    assert_eq!(
        hit.byte_index, 3,
        "right half of interior 'é' must place the caret after it (byte 3), got {}",
        hit.byte_index
    );
}

// ---------------------------------------------------------------------------
// caret_rect
// ---------------------------------------------------------------------------

#[test]
fn caret_rect_at_origin_has_positive_height() {
    let e = engine();
    let para = e.layout_paragraph("Hello", None).unwrap();
    let pos = HitPosition {
        byte_index: 0,
        affinity: TextAffinity::Downstream,
    };
    let rect = para.caret_rect(pos, 1.0);
    assert!(rect.height > 0.0, "caret height should be positive");
}

#[test]
fn caret_rect_width_matches_requested_caret_width() {
    // Width should approximately equal the requested caret_width (±1 px rounding).
    let e = engine();
    let para = e.layout_paragraph("Hello", None).unwrap();
    let pos = HitPosition {
        byte_index: 0,
        affinity: TextAffinity::Downstream,
    };

    let narrow = para.caret_rect(pos, 1.0);
    let wide = para.caret_rect(pos, 4.0);

    assert!(
        (narrow.width - 1.0).abs() <= 1.0,
        "1px caret width should be ~1px, got {}",
        narrow.width
    );
    assert!(
        (wide.width - 4.0).abs() <= 1.0,
        "4px caret width should be ~4px, got {}",
        wide.width
    );
    assert!(
        wide.width > narrow.width,
        "wider caret_width must produce wider rect ({} vs {})",
        wide.width,
        narrow.width
    );
}

#[test]
fn caret_at_end_is_to_the_right_of_start() {
    let e = engine();
    let text = "Hello";
    let para = e.layout_paragraph(text, None).unwrap();

    let start_pos = HitPosition {
        byte_index: 0,
        affinity: TextAffinity::Downstream,
    };
    let end_pos = HitPosition {
        byte_index: text.len(),
        affinity: TextAffinity::Upstream,
    };

    let start_rect = para.caret_rect(start_pos, 1.0);
    let end_rect = para.caret_rect(end_pos, 1.0);

    assert!(
        end_rect.origin.x > start_rect.origin.x,
        "end caret ({}) should be to the right of start caret ({})",
        end_rect.origin.x,
        start_rect.origin.x,
    );
}

// ---------------------------------------------------------------------------
// selection_rects
// ---------------------------------------------------------------------------

#[test]
fn collapsed_selection_returns_no_rects() {
    let e = engine();
    let para = e.layout_paragraph("Hello", None).unwrap();
    let pos = HitPosition {
        byte_index: 0,
        affinity: TextAffinity::Downstream,
    };
    let rects = para.selection_rects(pos, pos);
    assert!(
        rects.is_empty(),
        "collapsed selection should return no rects, got {}",
        rects.len()
    );
}

#[test]
fn full_selection_returns_at_least_one_rect() {
    let e = engine();
    let text = "Hello world";
    let para = e.layout_paragraph(text, None).unwrap();

    let anchor = HitPosition {
        byte_index: 0,
        affinity: TextAffinity::Downstream,
    };
    let focus = HitPosition {
        byte_index: text.len(),
        affinity: TextAffinity::Upstream,
    };

    let rects = para.selection_rects(anchor, focus);
    assert!(
        !rects.is_empty(),
        "full selection should produce at least one rect"
    );
}

#[test]
fn selection_rect_has_positive_area() {
    let e = engine();
    let text = "Hello";
    let para = e.layout_paragraph(text, None).unwrap();

    let anchor = HitPosition {
        byte_index: 0,
        affinity: TextAffinity::Downstream,
    };
    let focus = HitPosition {
        byte_index: text.len(),
        affinity: TextAffinity::Upstream,
    };

    let rects = para.selection_rects(anchor, focus);
    for r in &rects {
        assert!(r.width > 0.0, "selection rect must have positive width");
        assert!(r.height > 0.0, "selection rect must have positive height");
    }
}

#[test]
fn reversed_selection_produces_rects() {
    // Parley swaps anchor/focus internally. Prove our wrapper passes this through.
    let e = engine();
    let text = "Hello world";
    let para = e.layout_paragraph(text, None).unwrap();
    let start = HitPosition {
        byte_index: 0,
        affinity: TextAffinity::Downstream,
    };
    let end = HitPosition {
        byte_index: text.len(),
        affinity: TextAffinity::Upstream,
    };

    let forward = para.selection_rects(start, end);
    let backward = para.selection_rects(end, start);

    assert_eq!(
        forward.len(),
        backward.len(),
        "reversed selection must produce the same number of rects"
    );
    assert!(
        (forward[0].width - backward[0].width).abs() < 1.0,
        "rect widths should match: {} vs {}",
        forward[0].width,
        backward[0].width
    );
}

#[test]
fn wrapped_selection_returns_multiple_rects() {
    // Full-document selection over wrapped text should yield one rect per line.
    let e = engine();
    let text = "The quick brown fox jumps over the lazy dog";
    let para = e.layout_paragraph(text, Some(60.0)).unwrap();
    assert!(
        para.line_count() >= 2,
        "need at least 2 lines for this test"
    );

    let anchor = HitPosition {
        byte_index: 0,
        affinity: TextAffinity::Downstream,
    };
    let focus = HitPosition {
        byte_index: text.len(),
        affinity: TextAffinity::Upstream,
    };
    let rects = para.selection_rects(anchor, focus);

    assert!(
        rects.len() >= 2,
        "full selection over wrapped text should return >= 2 rects, got {}",
        rects.len()
    );
    // Rects on different lines must have distinct y-coordinates.
    let distinct_y: std::collections::HashSet<_> =
        rects.iter().map(|r| r.origin.y.to_bits()).collect();
    assert_eq!(
        distinct_y.len(),
        rects.len(),
        "each rect should be on a different line (distinct y)"
    );
}

#[test]
fn selection_rects_cover_expected_extent() {
    // The sum of rect widths should approximate the total paragraph width.
    // This proves selection_rects returns geometrically meaningful output,
    // not just a token non-empty vec.
    let e = engine();
    let text = "Hello world";
    let para = e.layout_paragraph(text, None).unwrap();

    let anchor = HitPosition {
        byte_index: 0,
        affinity: TextAffinity::Downstream,
    };
    let focus = HitPosition {
        byte_index: text.len(),
        affinity: TextAffinity::Upstream,
    };
    let rects = para.selection_rects(anchor, focus);

    assert!(!rects.is_empty());
    let total_width: f32 = rects.iter().map(|r| r.width).sum();
    let para_width = para.width();
    assert!(
        total_width >= para_width * 0.8,
        "selection rects total width ({total_width}) should cover most of paragraph width ({para_width})"
    );
}

// ---------------------------------------------------------------------------
// round-trip: hit_test → caret_rect
// ---------------------------------------------------------------------------

#[test]
fn hit_test_then_caret_rect_stays_on_screen() {
    let e = engine();
    let text = "Sphinx of black quartz";
    let para = e.layout_paragraph(text, Some(120.0)).unwrap();

    for x in [0.0_f32, 20.0, 40.0, 60.0, 80.0] {
        let pos = para.hit_test(x, 0.0);
        let rect = para.caret_rect(pos, 1.0);
        assert!(
            rect.origin.x >= 0.0,
            "caret x should be non-negative at sample x={x}"
        );
        assert!(
            rect.height > 0.0,
            "caret height should be positive at sample x={x}"
        );
    }
}
