#[cfg(all(test, feature = "scene"))]
#[path = "text/lib_tests.rs"]
mod tests;
#[cfg(all(feature = "scene", feature = "shaping-complex"))]
use tracing as _;
#[cfg(feature = "scene")]
use unicode_script as _;

#[cfg(feature = "scene")]
mod cache;
#[cfg(feature = "scene")]
mod engine;
#[cfg(feature = "scene")]
mod font_family;
pub(crate) mod font_settings;
#[cfg(all(feature = "scene", target_arch = "wasm32"))]
mod font_url;
#[cfg(scene_harness)]
#[doc(hidden)]
pub mod fonts;
#[cfg(scene_internal)]
pub(crate) mod fonts;
#[cfg(feature = "scene")]
mod numeric_run;
#[cfg(feature = "scene")]
pub(crate) mod outline;
#[cfg(feature = "scene")]
mod paragraph;
#[cfg(feature = "scene")]
mod script_fonts;
mod spans;
#[cfg(feature = "scene")]
mod system_fonts;
#[cfg(feature = "scene")]
pub(crate) mod web_font;

#[cfg(feature = "scene")]
pub(crate) mod color_glyph;
#[cfg(feature = "scene")]
mod colr_paint;
#[cfg(feature = "scene")]
mod colr_radial;
#[cfg(feature = "scene")]
pub(crate) mod glyph;

#[cfg(feature = "scene")]
pub(crate) mod shaper;

#[cfg(all(feature = "scene", feature = "shaping-complex"))]
pub(crate) mod shaping_complex;

pub use crate::grapheme::{byte_at_col, col_at_byte, col_count};
#[cfg(scene_harness)]
#[doc(hidden)]
pub use crate::hal::text::RenderedGlyph;
#[cfg(scene_internal)]
#[allow(unused_imports, reason = "public only under kittest")]
pub(crate) use crate::hal::text::RenderedGlyph;
pub use crate::hal::text::{HitPosition, TextAffinity};
pub use crate::layout::measure::TextMeasure;
pub use crate::layout::text_geometry::CaretGeometry;
pub use crate::style::TextStyle;
#[cfg(scene_internal)]
#[allow(unused_imports, reason = "public only under kittest")]
pub(crate) use engine::OwnedTextEngine;
#[cfg(scene_harness)]
#[doc(hidden)]
pub use engine::{FontCollection, OwnedTextEngine};
pub(crate) use font_settings::FontSettings;
#[cfg(all(feature = "scene", target_arch = "wasm32"))]
#[doc(hidden)]
pub use font_url::load_font_url;
#[cfg(all(feature = "scene", feature = "desktop"))]
pub(crate) use fonts::on_fonts_changed;
#[cfg(feature = "scene")]
pub(crate) use paragraph::FaceSpec;
#[cfg(scene_harness)]
#[doc(hidden)]
pub use paragraph::Paragraph;
#[cfg(scene_internal)]
#[allow(unused_imports, reason = "public only under kittest")]
pub(crate) use paragraph::Paragraph;
#[cfg(scene_harness)]
#[doc(hidden)]
pub use shaper::Font;
#[cfg(scene_internal)]
#[allow(unused_imports, reason = "public only under kittest")]
pub(crate) use shaper::Font;
pub use spans::Spans;
#[cfg(feature = "scene")]
pub(crate) use web_font::WebFont;
#[cfg(scene_internal)]
#[allow(unused_imports, reason = "public only under kittest")]
pub(crate) use web_font::set_web_fallbacks;
#[cfg(scene_harness)]
#[doc(hidden)]
pub use web_font::{set_web_fallbacks, web_fallbacks};

/// Measures or hit-tests text using the active frame's [`TextMeasure`] implementation.
#[must_use]
pub fn measure_text<R>(f: impl FnOnce(&dyn TextMeasure) -> R) -> Option<R> {
    crate::layout::with_text_geometry(f)
}
