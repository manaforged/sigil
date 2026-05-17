//! Owned shaper — skrifa-backed glyph layout. Body lands in G1.3.

// G1.1 stubs: signatures are frozen per DR-044 (`&self` on shape,
// non-`const` new) so the G1.3 body can land without churning the
// API surface. Suppress the stub-shaped lints until then.
#![allow(
    dead_code,
    clippy::redundant_pub_crate,
    clippy::missing_const_for_fn,
    clippy::unused_self
)]

use crate::glyph::GlyphRun;
use skrifa as _;

/// Placeholder font handle. G1.3 replaces with a skrifa-backed font
/// view that carries the loaded face + cmap + outline tables.
pub(crate) struct Font;

/// Placeholder text segment. G1.3 replaces with the byte-range + bidi
/// level + script tag produced by [`crate::segment`] and [`crate::bidi`].
pub(crate) struct TextSegment;

/// Placeholder style. G1.3 replaces with the resolved `TextStyle`
/// projection (family, weight, size, feature set).
pub(crate) struct Style;

/// Skrifa-backed shaper. G1.1 ships the type only — the shape body
/// lands in G1.3 against the property-test oracle.
pub(crate) struct Shaper {/* skrifa-backed; body lands in G1.3 */}

impl Shaper {
    /// Construct a shaper bound to `_font`.
    pub(crate) fn new(_font: &Font) -> Self {
        Self {}
    }

    /// Shape `_segment` under `_style`. Returns an empty run until
    /// G1.3 lands the real implementation.
    pub(crate) fn shape(&self, _segment: &TextSegment, _style: &Style) -> GlyphRun {
        GlyphRun::empty()
    }
}
