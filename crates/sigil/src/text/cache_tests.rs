use super::*;
use crate::style::TextLineHeight;
use crate::style::TextStyle;
use crate::text::OwnedTextEngine;

fn make_paragraph(text: &str, style: &TextStyle) -> Paragraph {
    let e = OwnedTextEngine::new(TextStyle {
        font_size: 16.0,
        ..TextStyle::default()
    });
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

fn seeded(text: &str, style: &TextStyle) -> ParagraphCache {
    let mut cache = ParagraphCache::new();
    cache.insert(
        text,
        None,
        style,
        CachedParagraph {
            paragraph: make_paragraph(text, style),
            bounds: WrapBounds::OPEN,
        },
    );
    cache
}

#[test]
fn identical_key_hits_with_identical_metrics() {
    let style = style_at(16.0);
    let cache = seeded("Typography", &style);
    let hit = cache
        .lookup("Typography", None, &style)
        .expect("identical key must hit the cache");
    let original_width = make_paragraph("Typography", &style).width();
    assert!((hit.width() - original_width).abs() < f32::EPSILON);
}

#[test]
fn differing_text_or_style_misses() {
    let style = style_at(16.0);
    let cache = seeded("Hello", &style);
    assert!(
        cache.lookup("Hello", None, &style_at(32.0)).is_none(),
        "differing style must miss the cache (same text, different size)",
    );
    assert!(
        cache.lookup("World", None, &style).is_none(),
        "different text must miss the cache"
    );
}

#[test]
fn collision_on_same_hashed_key_returns_miss_not_stale_layout() {
    let style = style_at(16.0);
    let mut cache = seeded("text-A", &style);

    let key = ParagraphCache::key("text-A", None, &style);
    if let Some(entry) = cache.map.get_mut(&key) {
        entry.text = "text-B".into();
    }

    let hit_a = cache.lookup("text-A", None, &style);
    assert!(
        hit_a.is_none(),
        "after a collision the verifier must reject the stale entry",
    );
}

#[test]
fn near_equal_widths_keep_separate_entries() {
    let mut cache = ParagraphCache::new();
    let style = style_at(16.0);
    cache.insert(
        "Hello",
        Some(100.2),
        &style,
        CachedParagraph {
            paragraph: make_paragraph("Hello", &style),
            bounds: WrapBounds {
                min_nowrap: 100.2,
                min_wrap: 100.3,
            },
        },
    );
    cache.insert(
        "Hello",
        Some(100.4),
        &style,
        CachedParagraph {
            paragraph: make_paragraph("Hello", &style),
            bounds: WrapBounds {
                min_nowrap: 100.4,
                min_wrap: 100.5,
            },
        },
    );
    assert!(cache.lookup("Hello", Some(100.2), &style).is_some());
    assert!(cache.lookup("Hello", Some(100.4), &style).is_some());
}
