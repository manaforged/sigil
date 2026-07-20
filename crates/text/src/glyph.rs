//! Glyph run output — the substrate Slug consumes.
//!
//! Types only. The shaper body in G1.3 populates `GlyphRun` from
//! skrifa outlines + cluster mappings. `Rect` here is intentionally
//! local: `sigil_hal::Rect` exists but uses an `origin: Point` shape,
//! not the flat `x/y/w/h` shape G1.5 needs for the Slug glyph-bbox
//! contract.

use smallvec::SmallVec;

/// Newtype for skrifa-native glyph id.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GlyphId(pub u16);

/// Single glyph position in a shaped run.
#[derive(Debug, Clone, Copy)]
pub struct GlyphPos {
    /// Glyph identity inside the source font.
    pub id: GlyphId,
    /// Horizontal advance applied after this glyph.
    pub advance: f32,
    /// Horizontal offset from the pen position before drawing.
    pub offset_x: f32,
    /// Vertical offset from the baseline before drawing.
    pub offset_y: f32,
    /// Byte index into the source text this glyph clusters with.
    pub cluster: u32,
}

/// Output of shaping a text segment.
#[derive(Debug, Clone)]
pub struct GlyphRun {
    /// Positioned glyphs in visual order.
    pub glyphs: SmallVec<[GlyphPos; 16]>,
    /// Baseline y-offset for this run.
    pub baseline: f32,
    /// Visual bounding box of the run.
    pub bbox: Rect,
}

impl GlyphRun {
    /// Construct an empty run. Used as a placeholder in the G1.1
    /// skeleton; G1.3 replaces all call sites with the real shape
    /// result.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            glyphs: SmallVec::new(),
            baseline: 0.0,
            bbox: Rect::zero(),
        }
    }
}

/// Plain rect for glyph bbox.
///
/// Distinct from [`sigil_hal::Rect`] (which carries an `origin: Point`
/// wrapper) so the Slug glyph-bbox surface can be the flat `x/y/w/h`
/// form it needs without an extra unwrap at the call site.
#[derive(Debug, Clone, Copy, Default)]
pub struct Rect {
    /// Left edge.
    pub x: f32,
    /// Top edge.
    pub y: f32,
    /// Width.
    pub w: f32,
    /// Height.
    pub h: f32,
}

impl Rect {
    /// Zero rect at origin.
    #[must_use]
    pub fn zero() -> Self {
        Self::default()
    }
}
