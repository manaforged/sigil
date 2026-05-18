//! Text engine trait — see [DR-004].
//!
//! The `TextEngine` trait abstracts paragraph layout, shaping, caret
//! modelling, and selection geometry. In production this is implemented
//! by `sigil-text` over skrifa + unicode-segmentation (DR-044).
//! Tests implement a monospace stub.
//!
//! Text editing operations (cursor movement, delete, undo/redo) and
//! IME integration are the responsibility of `sigil-text`, not of
//! this trait. The trait exposes only what downstream crates need to
//! lay out and measure text for layout, hit-testing, and painting.
//!
//! [DR-004]: ../../../docs/decisions/DR-004-text-parley.md

extern crate alloc;

use alloc::boxed::Box;
use core::error::Error;

use crate::geometry::Color;

/// A single positioned glyph ready for GPU rendering.
///
/// Produced by `sigil_text::Paragraph::rendered_glyphs`. The `font_id` is an
/// opaque u64 stable for the font blob's lifetime (derived from the blob's
/// internal atomic counter). Pass a matching `(font_id, &[u8], font_index)`
/// entry from `Paragraph::fonts()` so the renderer can preprocess outlines on
/// the first cache miss.
#[derive(Clone, Debug, PartialEq)]
pub struct RenderedGlyph {
    /// Opaque font identity — stable while the font blob is alive.
    pub font_id: u64,
    /// Font collection index (0 for single-font files).
    pub font_index: u32,
    /// Glyph identifier within the font (OpenType GID).
    pub glyph_id: u32,
    /// X position of the glyph origin in layout-local logical pixels.
    pub x: f32,
    /// Y position of the baseline in layout-local logical pixels.
    pub y: f32,
    /// Font size used for shaping, in logical pixels.
    pub size: f32,
    /// Glyph foreground colour.
    pub color: Color,
    /// Normalised variation coordinates for variable-font faces, encoded as
    /// raw fixed-point `i16` bits (`skrifa::instance::NormalizedCoord::to_bits`).
    ///
    /// Empty for static (non-variable) fonts. Must match the shaper's
    /// coordinates so the outline cache returns the correct contour for each
    /// variation instance.
    pub variation_coords: Box<[i16]>,
}

/// Which side of a cluster boundary the cursor logically sits on.
///
/// Relevant at bidi direction changes, explicit line breaks, and the
/// end of a line — positions where a single byte offset can map to
/// two visual locations.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub enum TextAffinity {
    /// Cursor is downstream — attached to the start of the following
    /// cluster. The default at most positions.
    #[default]
    Downstream,
    /// Cursor is upstream — attached to the end of the preceding
    /// cluster. Used after a hard line break or a bidi boundary.
    Upstream,
}

/// A resolved position within a laid-out paragraph.
///
/// Returned by hit-testing. Pass back to `Paragraph::caret_rect` or
/// `Paragraph::selection_rects_with` to produce screen geometry.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct HitPosition {
    /// UTF-8 byte offset into the source text string.
    ///
    /// Always points to a valid UTF-8 character boundary. This is a
    /// *byte* offset, not a char or grapheme-cluster count — use
    /// `text[..byte_index]` to slice the prefix, not `text.chars().nth()`.
    ///
    /// Out-of-range values passed to engine methods are silently clamped
    /// to the text end. Values returned by [`Paragraph::hit_test`] are
    /// always in-range and on a valid boundary.
    pub byte_index: usize,
    /// Which side of the cluster boundary this position is on.
    pub affinity: TextAffinity,
}

/// Read-only metrics from a laid-out paragraph.
pub trait ParagraphMetrics {
    /// Total width of the paragraph after line-breaking.
    fn width(&self) -> f32;

    /// Total height of the paragraph (sum of line heights).
    fn height(&self) -> f32;

    /// Number of lines after line-breaking.
    fn line_count(&self) -> usize;

    /// Minimum content width (narrowest the paragraph can be).
    fn min_content_width(&self) -> f32;

    /// Maximum content width (widest single line, unconstrained).
    fn max_content_width(&self) -> f32;
}

/// Paragraph layout and shaping abstraction.
pub trait TextEngine {
    /// Error type returned by text engine operations.
    type Error: Error + Send + Sync + 'static;

    /// Opaque handle to a laid-out paragraph.
    type Paragraph: ParagraphMetrics;

    /// Lay out a paragraph of text.
    ///
    /// `max_width` constrains line-breaking. `None` means unconstrained
    /// (single-line or natural wrap width).
    ///
    /// # Errors
    ///
    /// Returns `Self::Error` if paragraph layout fails.
    fn layout_paragraph(
        &self,
        text: &str,
        max_width: Option<f32>,
    ) -> Result<Self::Paragraph, Self::Error>;

    /// Clear any cached layout state. Called once per frame.
    fn clear_cache(&self) {}
}
