#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::float_cmp)]

use super::*;
use crate::text::shaper::Shaper;

const FIRA: &[u8] = sigil_fonts::FIRA_SANS_REGULAR;

fn font() -> Font {
    Font::from_bytes(FIRA.to_vec(), 0).unwrap()
}

#[test]
fn numeric_strings_match_the_full_shaper() {
    let font = font();
    let style = Style::regular(14.0);
    for text in ["12,345.67", "0.5", "-8.09 (3)", "100%", "14658.29"] {
        let seg = TextSegment::at_level(text, 0);
        let fast = shape(&seg, &style, &font, |s, st, f| {
            crate::text::shaping_complex::shape_full(s, st, f)
        })
        .expect("fast path");
        let full = Shaper::new(&font).shape(&seg, &style);
        let ids: Vec<_> = fast.glyphs.iter().map(|g| (g.id, g.cluster)).collect();
        let full_ids: Vec<_> = full.glyphs.iter().map(|g| (g.id, g.cluster)).collect();
        assert_eq!(ids, full_ids, "{text}: glyph ids and clusters");
        let width = |run: &GlyphRun| run.glyphs.iter().map(|g| g.advance).sum::<f32>();
        assert!(
            (width(&fast) - width(&full)).abs() < 0.01,
            "{text}: width {} vs {}",
            width(&fast),
            width(&full)
        );
        assert_eq!(fast.baseline, full.baseline, "{text}: baseline");
    }
}

#[test]
fn letters_and_rtl_take_the_full_path() {
    let font = font();
    let style = Style::regular(14.0);
    let full = |s: &TextSegment<'_>, st: &Style, f: &Font| {
        crate::text::shaping_complex::shape_full(s, st, f)
    };
    assert!(shape(&TextSegment::at_level("12 M", 0), &style, &font, full).is_none());
    assert!(shape(&TextSegment::at_level("123", 1), &style, &font, full).is_none());
    assert!(!is_simple(""));
}
