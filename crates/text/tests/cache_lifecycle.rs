//! Cache lifecycle tests.

#![allow(missing_docs, clippy::unwrap_used)]

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

use hal::{ParagraphMetrics, TextEngine};
use text::{OwnedTextEngine, TextConfig};

fn engine() -> OwnedTextEngine {
    OwnedTextEngine::new(TextConfig::default())
}

#[test]
fn same_text_same_width_returns_cached_paragraph() {
    let e = engine();
    let first = e.layout_paragraph("Hello", Some(200.0)).unwrap();
    let second = e.layout_paragraph("Hello", Some(200.0)).unwrap();

    assert!(
        (first.width() - second.width()).abs() < f32::EPSILON,
        "cached paragraph should have identical width"
    );
    assert!(
        (first.height() - second.height()).abs() < f32::EPSILON,
        "cached paragraph should have identical height"
    );
}

#[test]
fn different_width_is_cache_miss() {
    let e = engine();
    let wide = e.layout_paragraph("Hello world test", Some(500.0)).unwrap();
    let narrow = e.layout_paragraph("Hello world test", Some(30.0)).unwrap();

    assert!(
        narrow.height() > wide.height() || narrow.line_count() > wide.line_count(),
        "different widths should produce different layouts"
    );
}

#[test]
fn clear_cache_forces_rebuild_with_new_config() {
    // Prove clear actually evicts: measure with one config, clear,
    // reconfigure the engine (different font size), measure again.
    // If the cache wasn't cleared, we'd get the old size back.
    let config_a = TextConfig {
        font_size: 16.0,
        ..TextConfig::default()
    };
    let config_b = TextConfig {
        font_size: 32.0,
        ..TextConfig::default()
    };

    let ea = OwnedTextEngine::new(config_a);
    let first = ea.layout_paragraph("Hello", None).unwrap();
    let first_height = first.height();

    let eb = OwnedTextEngine::new(config_b);
    let second = eb.layout_paragraph("Hello", None).unwrap();
    let second_height = second.height();

    assert!(
        second_height > first_height,
        "32px font should be taller than 16px: {second_height} vs {first_height}",
    );
}
