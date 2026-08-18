//! Engine segmentation and owned-layout regression tests.

#![allow(missing_docs, clippy::unwrap_used, clippy::indexing_slicing)]

use ahash as _;
use criterion as _;
#[cfg(feature = "shaping-complex")]
use harfrust as _;
use image as _;
use sigil_kernel as _;
use sigil_layout as _;
use sigil_reactive as _;
use skrifa as _;
use smallvec as _;
use tracing as _;
use unicode_bidi as _;
use unicode_linebreak as _;
use unicode_script as _;
use unicode_segmentation as _;

use sigil_hal::{ParagraphMetrics, TextEngine};
use sigil_style::{TextLineHeight, TextStyle};
use sigil_text::{OwnedTextEngine, TextConfig, TextError};

mod engine_segmentation {
    use super::*;

    fn engine() -> OwnedTextEngine {
        OwnedTextEngine::new(TextConfig::default())
    }

    fn style_at(size: f32) -> TextStyle {
        TextStyle {
            font_family: String::from("system-ui"),
            font_size: size,
            font_weight: 400,
            line_height: TextLineHeight::FontSizeRelative(1.2),
            letter_spacing_em: 0.0,
            ..Default::default()
        }
    }

    #[test]
    fn empty_input_produces_empty_paragraph() {
        let e = engine();
        let p = e.layout_paragraph("", None).unwrap();
        assert_eq!(p.line_count(), 1, "empty paragraph still has one line");
        assert!(p.width() < f32::EPSILON, "empty width must be ~0");
    }

    #[test]
    fn single_line_ascii_has_one_line() {
        let e = engine();
        let p = e.layout_paragraph("Hello world", None).unwrap();
        assert_eq!(p.line_count(), 1);
        assert!(p.width() > 0.0);
    }

    #[test]
    fn wrapping_narrow_increases_line_count() {
        let e = engine();
        let wide = e.layout_paragraph("Hello world", None).unwrap();
        let narrow = e.layout_paragraph("Hello world", Some(30.0)).unwrap();
        assert!(
            narrow.line_count() > wide.line_count(),
            "expected more lines under tight constraint: {} vs {}",
            narrow.line_count(),
            wide.line_count(),
        );
    }

    #[test]
    fn multi_line_with_constraint_changes_height() {
        let e = engine();
        let wide = e.layout_paragraph("alpha beta gamma delta", None).unwrap();
        let narrow = e
            .layout_paragraph("alpha beta gamma delta", Some(40.0))
            .unwrap();
        assert!(
            narrow.height() > wide.height(),
            "wrapped paragraph must be taller",
        );
    }

    #[test]
    fn newline_produces_multiple_lines() {
        let e = engine();
        let p = e.layout_paragraph("hello\nworld", None).unwrap();
        assert_eq!(p.line_count(), 2, "LF must force a new line");
    }

    #[test]
    fn crlf_produces_multiple_lines() {
        let e = engine();
        let p = e.layout_paragraph("hello\r\nworld", None).unwrap();
        assert_eq!(p.line_count(), 2, "CRLF must force a new line");
    }

    #[test]
    fn classic_mac_cr_produces_multiple_lines() {
        let e = engine();
        let p = e.layout_paragraph("hello\rworld", None).unwrap();
        assert_eq!(p.line_count(), 2, "bare CR must force a new line");
    }

    #[test]
    fn unicode_line_separator_produces_multiple_lines() {
        let e = engine();
        let p = e.layout_paragraph("hello\u{2028}world", None).unwrap();
        assert_eq!(p.line_count(), 2, "U+2028 must force a new line");
    }

    #[test]
    fn paragraph_separator_produces_multiple_lines() {
        let e = engine();
        let p = e.layout_paragraph("hello\u{2029}world", None).unwrap();
        assert_eq!(p.line_count(), 2, "U+2029 must force a new line");
    }

    #[test]
    fn four_line_file_produces_four_lines() {
        let e = engine();
        let p = e
            .layout_paragraph("line1\nline2\nline3\nline4", None)
            .unwrap();
        assert_eq!(p.line_count(), 4);
    }

    #[test]
    fn newline_is_not_rendered_as_tofu() {
        let e = engine();
        let no_break = e.layout_paragraph("ab", None).unwrap();
        let with_break = e.layout_paragraph("a\nb", None).unwrap();
        assert_eq!(with_break.line_count(), 2);
        let max_line_width = with_break.width();
        assert!(
            max_line_width <= no_break.width() + 0.5,
            "broken paragraph max line width ({max_line_width}) must not exceed un-broken width ({})",
            no_break.width(),
        );
    }

    #[test]
    fn trailing_newline_produces_empty_final_line() {
        let e = engine();
        let p = e.layout_paragraph("hello\n", None).unwrap();
        assert_eq!(
            p.line_count(),
            2,
            "trailing LF must produce an empty final line",
        );
    }

    #[test]
    fn rtl_paragraph_produces_glyphs() {
        let e = engine();
        let p = e
            .layout_paragraph("\u{05D0}\u{05D1}\u{05D2}", None)
            .unwrap();
        assert!(p.width() > 0.0, "Hebrew must have positive width");
        let glyphs = p.rendered_glyphs();
        assert!(!glyphs.is_empty(), "Hebrew must produce glyphs");
    }

    #[test]
    fn mixed_bidi_paragraph_shapes_without_panic() {
        let e = engine();
        let p = e.layout_paragraph("abc\u{05D0}\u{05D1}xyz", None).unwrap();
        let glyphs = p.rendered_glyphs();
        assert!(glyphs.len() >= 6, "expected at least one glyph per cluster");
    }

    #[test]
    fn style_size_changes_paragraph_width() {
        let e = engine();
        let small = e
            .layout_paragraph_styled("Typography", None, &style_at(12.0))
            .unwrap();
        let large = e
            .layout_paragraph_styled("Typography", None, &style_at(48.0))
            .unwrap();
        assert!(
            large.width() > small.width(),
            "48px must be wider than 12px: {} vs {}",
            large.width(),
            small.width(),
        );
    }

    #[test]
    fn cache_returns_same_metrics_for_repeat_call() {
        let e = engine();
        let a = e.layout_paragraph("Hello", Some(200.0)).unwrap();
        let b = e.layout_paragraph("Hello", Some(200.0)).unwrap();
        assert!((a.width() - b.width()).abs() < f32::EPSILON);
        assert!((a.height() - b.height()).abs() < f32::EPSILON);
    }

    #[test]
    fn cache_invalidates_on_style_change() {
        let e = engine();
        let a = e
            .layout_paragraph_styled("Typography", None, &style_at(16.0))
            .unwrap();
        let b = e
            .layout_paragraph_styled("Typography", None, &style_at(32.0))
            .unwrap();
        assert!(
            (a.width() - b.width()).abs() > 1.0,
            "different sizes must miss cache",
        );
    }

    #[test]
    fn clear_cache_drops_entries() {
        let e = engine();
        let _ = e.layout_paragraph("Hello", None).unwrap();
        e.clear_cache();
        let after = e.layout_paragraph("Hello", None).unwrap();
        assert!(after.width() > 0.0);
    }

    fn segments_for_tests(text: &str) -> Vec<(u32, u32, u8, &'static str)> {
        engine().debug_segments_for_tests(text)
    }

    #[test]
    fn ltr_paragraph_all_segments_level_zero() {
        let segs = segments_for_tests("Hello world");
        assert!(!segs.is_empty(), "expected non-empty segment list");
        assert!(
            segs.iter().all(|(_, _, lvl, _)| *lvl == 0),
            "LTR-only paragraph must produce all-LTR segments, got {segs:?}",
        );
        assert!(
            segs.iter()
                .all(|(_, _, _, sc)| matches!(*sc, "Latin" | "Common")),
            "LTR Latin paragraph must classify as Latin/Common, got {segs:?}",
        );
    }

    #[test]
    fn rtl_only_paragraph_all_segments_rtl_level() {
        let text = "\u{0645}\u{0631}\u{062D}\u{0628}\u{0627} \u{0628}\u{0627}\u{0644}\u{0639}\u{0627}\u{0644}\u{0645}";
        let segs = segments_for_tests(text);
        assert!(!segs.is_empty(), "expected non-empty Arabic segments");
        let arabic_runs: Vec<_> = segs
            .iter()
            .filter(|(_, _, _, sc)| matches!(*sc, "Arabic"))
            .collect();
        assert!(
            !arabic_runs.is_empty(),
            "expected at least one Arabic-classified run in pure-RTL paragraph: {segs:?}",
        );
        assert!(
            arabic_runs.iter().all(|(_, _, lvl, _)| *lvl % 2 == 1),
            "every Arabic segment must sit at an odd bidi level in a pure-RTL paragraph, got {arabic_runs:?}",
        );
    }

    #[test]
    fn mixed_paragraph_emits_both_ltr_and_rtl_runs() {
        let text = "Hello \u{0645}\u{0631}\u{062D}\u{0628}\u{0627} world";
        let segs = segments_for_tests(text);
        let has_ltr_latin = segs
            .iter()
            .any(|(_, _, lvl, sc)| *lvl == 0 && matches!(*sc, "Latin"));
        let has_rtl_arabic = segs
            .iter()
            .any(|(_, _, lvl, sc)| *lvl % 2 == 1 && matches!(*sc, "Arabic"));
        assert!(
            has_ltr_latin,
            "mixed paragraph must emit at least one LTR Latin segment: {segs:?}",
        );
        assert!(
            has_rtl_arabic,
            "mixed paragraph must emit at least one RTL Arabic segment: {segs:?}",
        );
    }

    #[test]
    fn mixed_script_word_splits_at_script_boundary() {
        let text = "Hello\u{0645}\u{0631}\u{062D}";
        let segs = segments_for_tests(text);
        let latin_count = segs
            .iter()
            .filter(|(_, _, _, sc)| matches!(*sc, "Latin"))
            .count();
        let arabic_count = segs
            .iter()
            .filter(|(_, _, _, sc)| matches!(*sc, "Arabic"))
            .count();
        assert!(
            latin_count >= 1 && arabic_count >= 1,
            "expected at least one Latin and at least one Arabic segment in mixed-script word, got {segs:?}",
        );
        for w in segs.windows(2) {
            assert!(
                w[0].1 <= w[1].0,
                "segments must be byte-ordered and non-overlapping: {segs:?}",
            );
        }
    }

    #[test]
    fn register_font_accepts_valid_bytes() {
        const BYTES: &[u8] = include_bytes!("fixtures/FiraSans-Bold.ttf");
        let mut e = engine();
        let result = e.register_font(BYTES.to_vec(), 0);
        assert!(
            result.is_ok(),
            "register_font with valid bytes must succeed, got {result:?}",
        );
        let p = e.layout_paragraph("Hello", None).unwrap();
        assert!(p.width() > 0.0);
    }

    #[test]
    fn register_font_rejects_garbage_bytes() {
        let mut e = engine();
        let garbage = vec![0_u8; 64];
        let result = e.register_font(garbage, 0);
        assert!(
            result.is_err(),
            "register_font with non-font bytes must return TextError::InvalidFont, got {result:?}",
        );
        assert_eq!(result.unwrap_err(), TextError::InvalidFont);
    }

    #[test]
    fn mono_family_uses_different_face_than_sans() {
        let e = engine();
        let sans_style = TextStyle {
            font_family: String::from("sans"),
            ..style_at(16.0)
        };
        let mono_style = TextStyle {
            font_family: String::from("mono"),
            ..style_at(16.0)
        };
        let sans = e
            .layout_paragraph_styled("iiii", None, &sans_style)
            .unwrap();
        let mono = e
            .layout_paragraph_styled("iiii", None, &mono_style)
            .unwrap();
        assert!(
            mono.width() > sans.width(),
            "monospace `iiii` ({}) must be wider than proportional `iiii` ({})",
            mono.width(),
            sans.width(),
        );
    }

    #[test]
    fn unknown_family_falls_back_to_sans() {
        let e = engine();
        let unknown_style = TextStyle {
            font_family: String::from("does-not-exist"),
            ..style_at(16.0)
        };
        let sans_style = TextStyle {
            font_family: String::from("sans"),
            ..style_at(16.0)
        };
        let unknown = e
            .layout_paragraph_styled("hello", None, &unknown_style)
            .unwrap();
        let sans = e
            .layout_paragraph_styled("hello", None, &sans_style)
            .unwrap();
        assert!(
            (unknown.width() - sans.width()).abs() < f32::EPSILON,
            "unknown family must fall back to sans",
        );
    }

    #[test]
    fn register_font_swaps_in_new_face_for_layout() {
        const BOLD: &[u8] = include_bytes!("fixtures/FiraSans-Bold.ttf");

        let mut e = engine();
        e.register_font(BOLD.to_vec(), 0).unwrap();
        let p = e.layout_paragraph("Hello", None).unwrap();
        assert!(p.width() > 0.0, "post-swap layout must produce width");
        assert!(
            p.rendered_glyphs().iter().any(|g| g.glyph_id != 0),
            "post-swap layout must produce at least one non-notdef glyph",
        );
    }

    #[test]
    fn rtl_paragraph_segments_carry_level_into_glyphs() {
        let text = "\u{05D0}\u{05D1}\u{05D2}";
        let segs = segments_for_tests(text);
        assert!(
            segs.iter()
                .any(|(_, _, lvl, sc)| *lvl % 2 == 1 && matches!(*sc, "Hebrew")),
            "Hebrew paragraph must produce at least one RTL Hebrew segment: {segs:?}",
        );
    }
}
