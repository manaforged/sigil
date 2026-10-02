#![allow(
    clippy::redundant_pub_crate,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]

use std::cell::{Cell, OnceCell};
use std::rc::Rc;
use std::sync::Arc;

use crate::geometry::{Point, Rect};
use crate::hal::{HitPosition, ParagraphMetrics, TextAffinity};
use smallvec::SmallVec;

use crate::text::Font;
use crate::text::glyph::GlyphRun;

#[path = "paragraph_geometry.rs"]
mod geometry;
#[path = "paragraph_ink.rs"]
mod ink;

use geometry::{
    caret_x_in_line, hit_test_line, locate_line_for_position, order_positions, pick_line,
    select_line,
};

#[derive(Debug, Clone)]
pub(crate) struct LayoutLine {
    glyphs: SmallVec<[LineGlyph; 32]>,
    line_index: u32,
    width: f32,
    baseline_y: f32,
    height: f32,
    byte_start: u32,
    byte_end: u32,
}

impl LayoutLine {
    pub(crate) fn empty(line_index: u32, line_height: f32) -> Self {
        Self {
            glyphs: SmallVec::new(),
            line_index,
            width: 0.0,
            baseline_y: 0.0,
            height: line_height.max(1.0),
            byte_start: u32::MAX,
            byte_end: 0,
        }
    }

    pub(crate) const fn set_empty_byte_position(&mut self, byte: u32) {
        self.byte_start = byte;
        self.byte_end = byte;
    }

    pub(crate) fn append_segment(&mut self, seg: &SegmentAppend<'_>) {
        if seg.byte_start < self.byte_start {
            self.byte_start = seg.byte_start;
        }
        if seg.byte_end > self.byte_end {
            self.byte_end = seg.byte_end;
        }
        let mut pen_x = self.width;
        for g in &seg.run.glyphs {
            self.glyphs.push(LineGlyph {
                glyph_id: u32::from(g.id.0),
                x: pen_x + g.offset_x,
                advance: g.advance,
                offset_y: g.offset_y,
                cluster: g.cluster + seg.byte_start,
                level: seg.level,
                face_slot: seg.face_slot,
            });
            pen_x += g.advance;
        }
        self.width += seg.advance;
    }

    pub(crate) fn set_baseline_to(&mut self, line_height: f32, ascent: f32, descent: f32) {
        let lh = line_height.max(1.0);
        self.height = lh;
        let offset = if ascent > 0.0 {
            (lh - (ascent + descent)).mul_add(0.5, ascent)
        } else {
            lh * 0.8
        };
        #[allow(
            clippy::cast_precision_loss,
            reason = "pixel and char-count magnitudes are bounded; sub-unit precision is irrelevant at render scale"
        )]
        let idx = self.line_index as f32;
        self.baseline_y = idx.mul_add(lh, offset);
    }

    fn line_top(&self) -> f32 {
        #[allow(
            clippy::cast_precision_loss,
            reason = "pixel and char-count magnitudes are bounded; sub-unit precision is irrelevant at render scale"
        )]
        let idx = self.line_index as f32;
        idx * self.height
    }

    fn line_bottom(&self) -> f32 {
        self.line_top() + self.height
    }

    fn contains_y(&self, y: f32) -> bool {
        y >= self.line_top() && y < self.line_bottom()
    }

    const fn byte_range_contains(&self, byte: usize) -> bool {
        let b = u32_clamp(byte);
        b >= self.byte_start && b <= self.byte_end
    }
}

const fn u32_clamp(v: usize) -> u32 {
    if v > u32::MAX as usize {
        u32::MAX
    } else {
        v as u32
    }
}

#[derive(Debug, Clone, Copy)]
struct LineGlyph {
    glyph_id: u32,
    x: f32,
    advance: f32,
    offset_y: f32,
    cluster: u32,
    level: u8,
    face_slot: u16,
}

impl LineGlyph {
    const fn is_rtl(&self) -> bool {
        self.level & 1 == 1
    }

    fn leading_edge(&self) -> f32 {
        if self.is_rtl() {
            self.x + self.advance
        } else {
            self.x
        }
    }

    fn trailing_edge(&self) -> f32 {
        if self.is_rtl() {
            self.x
        } else {
            self.x + self.advance
        }
    }
}

pub(crate) struct SegmentAppend<'a> {
    pub(crate) run: &'a GlyphRun,
    pub(crate) advance: f32,
    pub(crate) byte_start: u32,
    pub(crate) byte_end: u32,
    pub(crate) level: u8,
    pub(crate) face_slot: u16,
}

#[derive(Clone, Copy, Default)]
struct ContentWidths {
    min: f32,
    max: f32,
}

#[derive(Clone, Copy)]
pub(crate) struct TextMetrics {
    pub font_size: f32,
    pub line_height: f32,
}

pub(crate) struct FaceEntry {
    bytes: Arc<Vec<u8>>,
    index: u32,
    id: u64,
    variation_coords: Box<[i16]>,
    skew: f32,
}

const OBLIQUE_ID: u64 = 1 << 63;

pub(crate) type FaceTable = Rc<[FaceEntry]>;

pub(crate) fn face_table(faces: &[FaceSpec], size_px: f32) -> FaceTable {
    faces
        .iter()
        .map(|face| FaceEntry::new(face, size_px))
        .collect()
}

impl FaceEntry {
    fn new(face: &FaceSpec, size_px: f32) -> Self {
        let oblique = face.skew.abs() > f32::EPSILON;
        Self {
            bytes: face.font.bytes_arc(),
            index: face.font.face_index(),
            id: face.font.id() ^ if oblique { OBLIQUE_ID } else { 0 },
            variation_coords: face.font.variation_coords(face.weight, size_px),
            skew: face.skew,
        }
    }
}

#[derive(Clone)]
pub(crate) struct FaceSpec {
    pub(crate) font: Font,
    pub(crate) weight: u16,
    pub(crate) skew: f32,
}

struct ParagraphInner {
    lines: Vec<LayoutLine>,
    font_size: f32,
    line_height: f32,
    fonts: FaceTable,
    min_content_hint: f32,
    content_widths: Cell<Option<ContentWidths>>,
    ink: OnceCell<Option<Rect>>,
}

#[derive(Clone)]
pub struct Paragraph {
    inner: Rc<ParagraphInner>,
}

impl Paragraph {
    pub(crate) fn new(
        lines: Vec<LayoutLine>,
        metrics: TextMetrics,
        fonts: FaceTable,
        min_content_hint: f32,
    ) -> Self {
        Self {
            inner: Rc::new(ParagraphInner {
                lines,
                font_size: metrics.font_size,
                line_height: metrics.line_height,
                fonts,
                min_content_hint,
                content_widths: Cell::new(None),
                ink: OnceCell::new(),
            }),
        }
    }

    fn content_widths(&self) -> ContentWidths {
        if let Some(cached) = self.inner.content_widths.get() {
            return cached;
        }
        let max = self
            .inner
            .lines
            .iter()
            .map(|l| l.width)
            .fold(0.0_f32, f32::max);
        let min = if self.inner.min_content_hint > 0.0 {
            self.inner.min_content_hint
        } else {
            max
        };
        let result = ContentWidths {
            min: min.min(max),
            max,
        };
        self.inner.content_widths.set(Some(result));
        result
    }

    #[must_use]
    pub fn hit_test(&self, x: f32, y: f32) -> HitPosition {
        if self.inner.lines.is_empty() {
            return HitPosition {
                byte_index: 0,
                affinity: TextAffinity::Downstream,
            };
        }
        let line = pick_line(&self.inner.lines, y);
        hit_test_line(line, x)
    }

    #[must_use]
    pub fn caret_rect(&self, pos: HitPosition, caret_width: f32) -> Rect {
        let line = locate_line_for_position(&self.inner.lines, pos);
        let Some(line) = line else {
            return Rect::new(
                Point::new(0.0, 0.0),
                caret_width.max(1.0),
                self.inner.line_height,
            );
        };
        let x = caret_x_in_line(line, pos);
        Rect::new(
            Point::new(x, line.line_top()),
            caret_width.max(1.0),
            line.height,
        )
    }

    pub fn selection_rects_with(
        &self,
        anchor: HitPosition,
        focus: HitPosition,
        mut f: impl FnMut(Rect),
    ) {
        let (lo, hi) = order_positions(anchor, focus);
        if lo.byte_index == hi.byte_index {
            return;
        }
        let (lo_b, hi_b) = (u32_clamp(lo.byte_index), u32_clamp(hi.byte_index));
        for (index, line) in self.inner.lines.iter().enumerate() {
            select_line(
                line,
                self.inner.lines.get(index + 1),
                (lo_b, hi_b),
                (self.inner.font_size * 0.5).max(1.0),
                &mut f,
            );
        }
    }

    #[must_use]
    pub fn selection_rects(&self, anchor: HitPosition, focus: HitPosition) -> Vec<Rect> {
        let mut rects = Vec::new();
        self.selection_rects_with(anchor, focus, |r| rects.push(r));
        rects
    }

    pub fn for_each_font<F>(&self, mut f: F)
    where
        F: FnMut(u64, &[u8], u32),
    {
        self.for_each_font_arc(|id, bytes, index| f(id, bytes.as_slice(), index));
    }

    pub(crate) fn for_each_font_arc<F>(&self, mut f: F)
    where
        F: FnMut(u64, &Arc<Vec<u8>>, u32),
    {
        if self.inner.lines.iter().all(|l| l.glyphs.is_empty()) {
            return;
        }
        for (index, face) in self.inner.fonts.iter().enumerate() {
            let seen = self
                .inner
                .fonts
                .get(..index)
                .unwrap_or_default()
                .iter()
                .any(|prior| prior.id == face.id && prior.index == face.index);
            if !seen {
                f(face.id, &face.bytes, face.index);
            }
        }
    }
}

impl ParagraphMetrics for Paragraph {
    fn width(&self) -> f32 {
        self.inner
            .lines
            .iter()
            .map(|l| l.width)
            .fold(0.0_f32, f32::max)
    }

    fn height(&self) -> f32 {
        self.inner.lines.iter().map(|l| l.height).sum()
    }

    fn line_count(&self) -> usize {
        self.inner.lines.len()
    }

    fn min_content_width(&self) -> f32 {
        self.content_widths().min
    }

    fn max_content_width(&self) -> f32 {
        self.content_widths().max
    }
}
