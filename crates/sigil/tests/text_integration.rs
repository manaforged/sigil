#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]

#[path = "text_integration/bidi_visual_order.rs"]
mod bidi_visual_order;
#[path = "text_integration/cache_lifecycle.rs"]
mod cache_lifecycle;
#[path = "text_integration/ellipsize.rs"]
mod ellipsize;
#[path = "text_integration/empty_lines.rs"]
mod empty_lines;
#[path = "text_integration/font_fallback.rs"]
mod font_fallback;
#[path = "text_integration/hit_test.rs"]
mod hit_test;
#[path = "text_integration/measurement.rs"]
mod measurement;
#[path = "text_integration/min_max_content.rs"]
mod min_max_content;
#[path = "text_integration/reactive_text.rs"]
mod reactive_text;
#[path = "text_integration/rtl_caret_geometry.rs"]
mod rtl_caret_geometry;
#[path = "text_integration/engine_segmentation.rs"]
mod segmentation;
