//! Visual bidi reorder integration tests (Wave 4, W4.3).
//!
//! `crates/text/src/bidi.rs` computes UAX #9 embedding levels; these
//! tests prove the ENGINE consumes them to lay a line out in UAX #9
//! rule L2 visual order, not source order — the gap the
//! `docs/followups.md` "G2 visual bidi reorder" entry tracked.

#![allow(missing_docs, clippy::unwrap_used, clippy::indexing_slicing)]

use ahash as _;
use criterion as _;
#[cfg(feature = "shaping-complex")]
use harfrust as _;
use sigil_kernel as _;
use sigil_layout as _;
use sigil_reactive as _;
use sigil_style as _;
use skrifa as _;
use smallvec as _;
use tracing as _;
use unicode_bidi as _;
use unicode_linebreak as _;
use unicode_script as _;
use unicode_segmentation as _;

use sigil_hal::{ParagraphMetrics, TextEngine};
use sigil_text::{OwnedTextEngine, TextConfig};

/// SHIN LAMED VAV MEM-FINAL — "שלום" (Shalom). 4 chars, 8 UTF-8 bytes.
const WORD_A: &str = "\u{05E9}\u{05DC}\u{05D5}\u{05DD}";
/// AYIN VAV LAMED MEM — "עולם" (Olam). 4 chars, 8 UTF-8 bytes.
const WORD_B: &str = "\u{05E2}\u{05D5}\u{05DC}\u{05DD}";

fn engine() -> OwnedTextEngine {
    OwnedTextEngine::new(TextConfig::default())
}

/// Caret x for `(byte_index, affinity)`, to keep call sites terse.
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
    // "abc אבג def": a single embedded RTL run inside an LTR-base
    // paragraph. The Hebrew segment must sit physically between "abc"
    // and "def" (not before or after both), while its own glyphs
    // advance right-to-left internally (HarfRust's per-run visual
    // order, proven separately in shaping_complex::tests).
    let e = engine();
    let text = "abc \u{05D0}\u{05D1}\u{05D2} def";
    let p = e.layout_paragraph(text, None).unwrap();
    assert!(!p.rendered_glyphs().is_empty());

    // Hebrew source bytes: "abc " is 4 bytes (0..4), Alef/Bet/Gimel
    // occupy bytes 4..10 (2 bytes each), then " def" follows. Caret
    // geometry is cluster-addressable, so it locates each block's
    // physical x span directly.
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
    // Two Hebrew words separated by a space, all at the same RTL
    // embedding level — the case that distinguishes a real L2 reorder
    // from "HarfRust already reverses glyphs within a run": with NO
    // segment-level reorder, word A (first in source order) would sit
    // leftmost (pen_x starts at 0) and word B rightmost. UAX #9 says
    // the opposite: word A is read FIRST in Hebrew (right-to-left), so
    // it must be RIGHTMOST on screen; word B (read second) leftmost.
    let e = engine();
    let text = format!("{WORD_A} {WORD_B}");
    let p = e.layout_paragraph(&text, None).unwrap();

    // Leftmost hit (x=0) must land inside word B (the visually-first,
    // i.e. leftmost, word) — its byte range starts after WORD_A + the
    // space (9 bytes: 8 + 1).
    let word_b_start = u32::try_from(WORD_A.len() + 1).unwrap();
    let leftmost_hit = p.hit_test(0.0, 0.0);
    assert!(
        leftmost_hit.byte_index >= word_b_start as usize,
        "leftmost x must land in word B (visually first in an RTL paragraph), \
         got byte_index {} (word B starts at {word_b_start})",
        leftmost_hit.byte_index,
    );

    // Rightmost hit must land inside word A (visually last). Sample
    // just short of `p.width()` — `hit_test` fast-paths `x >=
    // line.width` straight to `byte_end` regardless of visual order,
    // which isn't the invariant under test here (that fast path is
    // itself correct; it just isn't glyph-geometry-driven).
    let word_a_end = u32::try_from(WORD_A.len()).unwrap();
    let rightmost_hit = p.hit_test(p.width() - 1.0, 0.0);
    assert!(
        rightmost_hit.byte_index <= word_a_end as usize,
        "rightmost x must land in word A (visually last in an RTL paragraph), \
         got byte_index {} (word A ends at {word_a_end})",
        rightmost_hit.byte_index,
    );

    // Round-trip hit_test → caret_rect must stay self-consistent over
    // the reordered line: the caret for the leftmost hit must sit near
    // the left edge, and the caret for the near-rightmost hit must sit
    // further right than it — proving `caret_rect` paints where
    // `hit_test` says the click landed, on a visually-reordered line.
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
    // "אבג abc def" (RTL Hebrew word + two LTR Latin words) wrapped so
    // "def" lands on its own line. The space BETWEEN "abc" and "def"
    // has a Latin neighbour on both sides, so whole-paragraph bidi
    // resolution matches it to the Latin run's level (2) — correct
    // only if the line keeps running into "def". Once "def" wraps
    // away, UAX #9 rule L1 requires resetting that now-line-final
    // space to the paragraph level (1) instead, which changes the L2
    // visual order: the trailing space leads the line instead of
    // "abc". Verified empirically (crates/text/src/bidi.rs unit tests
    // + a manual before/after run of this exact scenario) that feeding
    // raw paragraph-wide levels into L2 — the pre-fix behaviour —
    // instead puts "abc" flush at the line's left edge (x=0).
    let e = engine();
    let text = "\u{05D0}\u{05D1}\u{05D2} abc def";
    let p = e.layout_paragraph(text, Some(70.0)).unwrap();
    assert_eq!(
        p.line_count(),
        2,
        "expected width=70 to wrap 'def' onto its own line, got {} lines",
        p.line_count(),
    );

    // Byte 7 is 'a' — the start of "abc" (Hebrew word = bytes 0..6,
    // space = byte 6). `caret_x_for_byte` resolves by exact cluster
    // match (order-independent), so this reads 'a's real visual x
    // regardless of where it landed in the reordered glyph array.
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
    // Wider fixture: multiple RTL/LTR words with trailing spaces and
    // neutrals, wrapped to 3+ visual lines. Each wrap point puts a
    // DIFFERENT run's trailing whitespace at a line boundary, so this
    // exercises `ParagraphBidi::visual_order_for_line`'s L1 reset
    // across several distinct line-cut positions in one paragraph
    // (not just the single hand-picked byte offset in the test above).
    let e = engine();
    let text = format!("{WORD_A} abc {WORD_B} def {WORD_A} ghi");
    let p = e.layout_paragraph(&text, Some(45.0)).unwrap();
    assert!(
        p.line_count() >= 3,
        "expected width=45 to wrap into 3+ lines, got {}",
        p.line_count(),
    );

    // Every line must produce sane, finite, non-negative-width glyph
    // geometry, and hit_test -> caret_rect must round-trip without
    // panicking anywhere along the line (the round-trip invariant
    // hit_test.rs already pins for a simpler 2-run case).
    for line in 0..p.line_count() {
        // `line_count()` is a handful of lines for this fixture — the
        // usize -> f32 cast is exact at this scale.
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
