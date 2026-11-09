extern crate alloc;

use alloc::boxed::Box;

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
