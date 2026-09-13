extern crate alloc;

use alloc::boxed::Box;
use core::error::Error;

use crate::hal::geometry::Color;

#[derive(Clone, Debug, PartialEq)]
pub struct RenderedGlyph {
    pub font_id: u64,
    pub font_index: u32,
    pub glyph_id: u32,
    pub x: f32,
    pub y: f32,
    pub size: f32,
    pub color: Color,
    pub variation_coords: Box<[i16]>,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub enum TextAffinity {
    #[default]
    Downstream,
    Upstream,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct HitPosition {
    pub byte_index: usize,
    pub affinity: TextAffinity,
}

pub trait ParagraphMetrics {
    fn width(&self) -> f32;

    fn height(&self) -> f32;

    fn line_count(&self) -> usize;

    fn min_content_width(&self) -> f32;

    fn max_content_width(&self) -> f32;
}

pub trait TextEngine {
    type Error: Error + Send + Sync + 'static;

    type Paragraph: ParagraphMetrics;

    fn layout_paragraph(
        &self,
        text: &str,
        max_width: Option<f32>,
    ) -> Result<Self::Paragraph, Self::Error>;

    fn clear_cache(&self) {}
}
