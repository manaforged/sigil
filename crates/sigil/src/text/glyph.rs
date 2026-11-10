use smallvec::SmallVec;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct GlyphId(pub(crate) u16);

#[derive(Debug, Clone, Copy)]
pub(crate) struct GlyphPos {
    pub(crate) id: GlyphId,
    pub(crate) advance: f32,
    pub(crate) offset_x: f32,
    pub(crate) offset_y: f32,
    pub(crate) cluster: u32,
}

#[derive(Debug, Clone)]
#[cfg_attr(not(test), allow(dead_code, reason = "only tests read the run totals"))]
pub(crate) struct ShapedRun {
    pub(crate) glyphs: SmallVec<[GlyphPos; 16]>,
    pub(crate) baseline: f32,
    pub(crate) width: f32,
}

impl ShapedRun {
    #[must_use]
    pub(crate) fn empty() -> Self {
        Self {
            glyphs: SmallVec::new(),
            baseline: 0.0,
            width: 0.0,
        }
    }
}
