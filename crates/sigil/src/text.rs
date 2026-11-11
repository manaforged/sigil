#[cfg(test)]
#[path = "text/lib_tests.rs"]
mod tests;
#[cfg(feature = "shaping-complex")]
use tracing as _;
use unicode_script as _;

mod cache;
mod engine;
mod font_family;
#[cfg(target_arch = "wasm32")]
mod font_url;
pub(crate) mod fonts;
mod numeric_run;
mod paragraph;
mod script_fonts;
mod system_fonts;
mod web_font;

pub(crate) mod color_glyph;
mod colr_paint;
mod colr_radial;
pub(crate) mod glyph;

pub(crate) mod shaper;

#[cfg(feature = "shaping-complex")]
pub(crate) mod shaping_complex;

pub use crate::hal::text::{HitPosition, RenderedGlyph, TextAffinity};
#[cfg(feature = "kittest")]
pub use engine::FontCollection;
pub use engine::OwnedTextEngine;
#[cfg(target_arch = "wasm32")]
pub use font_url::load_font_url;
#[cfg(feature = "desktop")]
pub(crate) use fonts::on_fonts_changed;
pub(crate) use paragraph::FaceSpec;
pub use paragraph::Paragraph;
pub use shaper::Font;
#[cfg(feature = "kittest")]
#[doc(hidden)]
pub use web_font::web_fallbacks;
pub use web_font::{WebFont, WebFontScript, set_web_fallbacks};
