use smallvec::SmallVec;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GlyphId(pub u16);

#[derive(Debug, Clone, Copy)]
pub struct GlyphPos {
    pub id: GlyphId,
    pub advance: f32,
    pub offset_x: f32,
    pub offset_y: f32,
    pub cluster: u32,
}

#[derive(Debug, Clone)]
pub struct GlyphRun {
    pub glyphs: SmallVec<[GlyphPos; 16]>,
    pub baseline: f32,
    pub bbox: Rect,
}

impl GlyphRun {
    #[must_use]
    pub fn empty() -> Self {
        Self {
            glyphs: SmallVec::new(),
            baseline: 0.0,
            bbox: Rect::zero(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    #[must_use]
    pub fn zero() -> Self {
        Self::default()
    }
}
