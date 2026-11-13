extern crate alloc;

use alloc::boxed::Box;

use crate::hal::geometry::Rgba;

#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct RenderedGlyph {
    pub font_id: u64,
    pub font_index: u32,
    pub glyph_id: u32,
    pub x: f32,
    pub y: f32,
    pub size: f32,
    pub color: Rgba,
    pub variation_coords: Box<[i16]>,
    pub skew: f32,
}

/// Which side of a line break a text position belongs to.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
#[non_exhaustive]
pub enum TextAffinity {
    /// At a soft line break, the position is at the start of the next line.
    #[default]
    Downstream,
    /// At a soft line break, the position is at the end of the line before the break.
    Upstream,
}

/// The text position found under a point.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct HitPosition {
    /// The byte offset of the position in the text.
    pub byte_index: usize,
    /// The line that the position belongs to when it falls on a soft line break.
    pub affinity: TextAffinity,
}
