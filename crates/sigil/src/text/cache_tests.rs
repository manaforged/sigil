use super::*;
use crate::hal::ParagraphMetrics;
use crate::text::OwnedTextEngine;
use crate::text::config::TextConfig;

fn make_paragraph(text: &str, style: &TextStyle) -> Paragraph {
    let e = OwnedTextEngine::new(TextConfig::default());
    e.layout_paragraph_styled(text, None, style).unwrap()
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
fn same_text_same_style_returns_cached_entry() {
    let mut cache = ParagraphCache::new();
    let style = style_at(16.0);
    let p = make_paragraph("Hello", &style);
    cache.insert("Hello", None, &style, p);

    let hit = cache.get("Hello", None, &style);
    assert!(hit.is_some(), "identical key must hit the cache");
}

#[test]
fn different_style_misses_even_when_text_matches() {
    let mut cache = ParagraphCache::new();
    let style_a = style_at(16.0);
    let style_b = style_at(32.0);
    let p = make_paragraph("Hello", &style_a);
    cache.insert("Hello", None, &style_a, p);

    let hit = cache.get("Hello", None, &style_b);
    assert!(
        hit.is_none(),
        "differing style must miss the cache (same text, different size)",
    );
}

#[test]
fn collision_on_same_hashed_key_returns_miss_not_stale_layout() {
    let mut cache = ParagraphCache::new();
    let style = style_at(16.0);
    let p_a = make_paragraph("text-A", &style);
    cache.insert("text-A", None, &style, p_a);

    let key = ParagraphCache::key("text-A", None, &style);
    if let Some(entry) = cache.map.get_mut(&key) {
        entry.text = String::from("text-B");
    }

    let hit_a = cache.get("text-A", None, &style);
    assert!(
        hit_a.is_none(),
        "after a collision the verifier must reject the stale entry",
    );
}

#[test]
fn different_text_misses_even_when_style_matches() {
    let mut cache = ParagraphCache::new();
    let style = style_at(16.0);
    let p = make_paragraph("Hello", &style);
    cache.insert("Hello", None, &style, p);

    let hit = cache.get("World", None, &style);
    assert!(hit.is_none(), "different text must miss the cache");
}

#[test]
fn cached_paragraph_metrics_are_identical_on_hit() {
    let mut cache = ParagraphCache::new();
    let style = style_at(16.0);
    let p = make_paragraph("Typography", &style);
    let original_width = p.width();
    cache.insert("Typography", None, &style, p);

    let hit = cache.get("Typography", None, &style).unwrap();
    assert!((hit.width() - original_width).abs() < f32::EPSILON);
}
