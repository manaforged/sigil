extern crate alloc;

use alloc::boxed::Box;

use crate::hal::geometry::Rgba;

#[doc(hidden)]
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
#[must_use]
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct HitPosition {
    /// The byte offset of the position in the text.
    pub(crate) byte_index: usize,
    /// The line that the position belongs to when it falls on a soft line break.
    pub(crate) affinity: TextAffinity,
}

impl HitPosition {
    /// Creates a hit position at `byte_index` with `affinity`.
    pub const fn new(byte_index: usize, affinity: TextAffinity) -> Self {
        Self {
            byte_index,
            affinity,
        }
    }

    /// Creates a downstream hit position at `byte_index`.
    pub const fn downstream(byte_index: usize) -> Self {
        Self::new(byte_index, TextAffinity::Downstream)
    }

    /// Returns the UTF-8 byte offset of the hit position.
    #[must_use]
    pub const fn byte_index(&self) -> usize {
        self.byte_index
    }

    /// Returns the line affinity when the offset falls on a soft line break.
    #[must_use]
    pub const fn affinity(&self) -> TextAffinity {
        self.affinity
    }
}
