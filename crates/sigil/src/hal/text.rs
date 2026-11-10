extern crate alloc;

use alloc::boxed::Box;
use core::error::Error;

use crate::hal::geometry::Color;

#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct RenderedGlyph {
    pub font_id: u64,
    pub font_index: u32,
    pub glyph_id: u32,
    pub x: f32,
    pub y: f32,
    pub size: f32,
    pub color: Color,
    pub variation_coords: Box<[i16]>,
    pub skew: f32,
}

/// Which side of a line break a text position belongs to.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
#[non_exhaustive]
pub enum TextAffinity {
    /// The downstream text affinity.
    #[default]
    Downstream,
    /// The upstream text affinity.
    Upstream,
}

/// The text position found under a point.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct HitPosition {
    /// The byte index.
    pub byte_index: usize,
    /// The affinity.
    pub affinity: TextAffinity,
}

#[cfg(any(test, feature = "kittest", feature = "scene"))]
pub trait ParagraphMetrics {
    fn width(&self) -> f32;

    fn height(&self) -> f32;

    #[cfg(any(test, feature = "kittest"))]
    fn line_count(&self) -> usize;

    fn min_content_width(&self) -> f32;

    #[cfg(any(test, feature = "kittest"))]
    fn max_content_width(&self) -> f32;
}

#[cfg(any(test, feature = "kittest", feature = "scene"))]
pub trait TextEngine {
    type Error: Error + Send + Sync + 'static;

    type Paragraph: ParagraphMetrics;

    #[cfg(any(test, feature = "kittest"))]
    fn layout_paragraph(
        &self,
        text: &str,
        max_width: Option<f32>,
    ) -> Result<Self::Paragraph, Self::Error>;

    #[cfg(any(test, feature = "kittest"))]
    fn clear_cache(&self) {}
}
