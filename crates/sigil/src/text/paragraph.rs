#![allow(
    clippy::redundant_pub_crate,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]

use std::cell::Cell;
use std::rc::Rc;
use std::sync::Arc;

use crate::hal::{Color, HitPosition, ParagraphMetrics, Point, Rect, RenderedGlyph, TextAffinity};
use smallvec::SmallVec;

use crate::text::Font;
use crate::text::glyph::GlyphRun;

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

struct FaceEntry {
    bytes: Arc<Vec<u8>>,
    index: u32,
    id: u64,
    variation_coords: Box<[i16]>,
}

impl FaceEntry {
    fn new(font: &Font, weight: u16, size_px: f32) -> Self {
        Self {
            bytes: font.bytes_arc(),
            index: font.face_index(),
            id: font.id(),
            variation_coords: font.variation_coords(weight, size_px),
        }
    }
}

struct ParagraphInner {
    lines: Vec<LayoutLine>,
    font_size: f32,
    line_height: f32,
    fonts: Vec<FaceEntry>,
    min_content_hint: f32,
    content_widths: Cell<Option<ContentWidths>>,
}

#[derive(Clone)]
pub struct Paragraph {
    inner: Rc<ParagraphInner>,
}

impl Paragraph {
    pub(crate) fn new(
        lines: Vec<LayoutLine>,
        metrics: TextMetrics,
        faces: &[Font],
        weight: u16,
        min_content_hint: f32,
    ) -> Self {
        let fonts: Vec<FaceEntry> = faces
            .iter()
            .map(|font| FaceEntry::new(font, weight, metrics.font_size))
            .collect();
        Self {
            inner: Rc::new(ParagraphInner {
                lines,
                font_size: metrics.font_size,
                line_height: metrics.line_height,
                fonts,
                min_content_hint,
                content_widths: Cell::new(None),
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
        let line = locate_line_for_byte(&self.inner.lines, pos.byte_index);
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
        for line in &self.inner.lines {
            let mut span: Option<(f32, f32)> = None;
            for g in &line.glyphs {
                if g.cluster >= lo_b && g.cluster < hi_b {
                    let (gl, gr) = (g.x, g.x + g.advance);
                    span = Some(span.map_or((gl, gr), |(l, r)| (l.min(gl), r.max(gr))));
                } else {
                    emit_span(span.take(), line, &mut f);
                }
            }
            emit_span(span.take(), line, &mut f);
        }
    }

    #[must_use]
    pub fn selection_rects(&self, anchor: HitPosition, focus: HitPosition) -> Vec<Rect> {
        let mut rects = Vec::new();
        self.selection_rects_with(anchor, focus, |r| rects.push(r));
        rects
    }

    #[must_use]
    pub fn rendered_glyphs(&self) -> Vec<RenderedGlyph> {
        let mut out = Vec::new();
        for line in &self.inner.lines {
            for g in &line.glyphs {
                let face = self.face_for_slot(g.face_slot);
                out.push(RenderedGlyph {
                    font_id: face.id,
                    font_index: face.index,
                    glyph_id: g.glyph_id,
                    x: g.x,
                    y: line.baseline_y + g.offset_y,
                    size: self.inner.font_size,
                    color: Color::WHITE,
                    variation_coords: face.variation_coords.clone(),
                });
            }
        }
        out
    }

    fn face_for_slot(&self, slot: u16) -> &FaceEntry {
        self.inner
            .fonts
            .get(slot as usize)
            .or_else(|| self.inner.fonts.first())
            .expect("Paragraph::new always seeds at least the primary face")
    }

    pub fn for_each_font<F>(&self, mut f: F)
    where
        F: FnMut(u64, &[u8], u32),
    {
        if self.inner.lines.iter().all(|l| l.glyphs.is_empty()) {
            return;
        }
        for face in &self.inner.fonts {
            f(face.id, face.bytes.as_slice(), face.index);
        }
    }
}

fn pick_line(lines: &[LayoutLine], y: f32) -> &LayoutLine {
    for l in lines {
        if l.contains_y(y) {
            return l;
        }
    }
    if y < 0.0 {
        lines.first().expect("non-empty (caller guards)")
    } else {
        lines.last().expect("non-empty (caller guards)")
    }
}

fn locate_line_for_byte(lines: &[LayoutLine], byte_index: usize) -> Option<&LayoutLine> {
    for l in lines {
        if l.byte_range_contains(byte_index) {
            return Some(l);
        }
    }
    lines.last()
}

fn hit_test_line(line: &LayoutLine, x: f32) -> HitPosition {
    if line.glyphs.is_empty() {
        return HitPosition {
            byte_index: line.byte_start.min(line.byte_end) as usize,
            affinity: TextAffinity::Downstream,
        };
    }
    if x <= 0.0
        && let Some(g) = line.glyphs.first()
    {
        return edge_hit(line, g, true);
    }
    if x >= line.width
        && let Some(g) = line.glyphs.last()
    {
        return edge_hit(line, g, false);
    }
    let Some(best) = pick_nearest_glyph(&line.glyphs, x) else {
        return HitPosition {
            byte_index: line.byte_start as usize,
            affinity: TextAffinity::Downstream,
        };
    };
    let mid = best.advance.mul_add(0.5, best.x);
    let near_leading = if best.is_rtl() { x >= mid } else { x < mid };
    let (byte_index, affinity) = if near_leading {
        (best.cluster as usize, TextAffinity::Downstream)
    } else {
        (
            next_cluster_after(line, best.cluster) as usize,
            TextAffinity::Upstream,
        )
    };
    HitPosition {
        byte_index,
        affinity,
    }
}

fn edge_hit(line: &LayoutLine, g: &LineGlyph, visual_left: bool) -> HitPosition {
    let leading_is_left = !g.is_rtl();
    if visual_left == leading_is_left {
        HitPosition {
            byte_index: g.cluster as usize,
            affinity: TextAffinity::Downstream,
        }
    } else {
        HitPosition {
            byte_index: next_cluster_after(line, g.cluster) as usize,
            affinity: TextAffinity::Upstream,
        }
    }
}

fn pick_nearest_glyph(glyphs: &[LineGlyph], x: f32) -> Option<&LineGlyph> {
    let mut iter = glyphs.iter();
    let mut best = iter.next()?;
    let mut best_dx = (best.x - x).abs();
    for g in iter {
        let dx = (g.x - x).abs();
        if dx < best_dx {
            best_dx = dx;
            best = g;
        }
    }
    Some(best)
}

fn next_cluster_after(line: &LayoutLine, cluster: u32) -> u32 {
    line.glyphs
        .iter()
        .map(|glyph| glyph.cluster)
        .filter(|&candidate| candidate > cluster)
        .min()
        .unwrap_or(line.byte_end)
}

fn emit_span(span: Option<(f32, f32)>, line: &LayoutLine, f: &mut impl FnMut(Rect)) {
    if let Some((l, r)) = span
        && r > l
    {
        f(Rect::new(
            Point::new(l, line.line_top()),
            r - l,
            line.height,
        ));
    }
}

fn caret_x_in_line(line: &LayoutLine, pos: HitPosition) -> f32 {
    if line.glyphs.is_empty() {
        return 0.0;
    }
    let target = u32_clamp(pos.byte_index);
    match pos.affinity {
        TextAffinity::Downstream => line
            .glyphs
            .iter()
            .find(|g| g.cluster == target)
            .or_else(|| {
                line.glyphs
                    .iter()
                    .filter(|g| g.cluster > target)
                    .min_by_key(|g| g.cluster)
            })
            .map_or_else(
                || {
                    line.glyphs
                        .iter()
                        .max_by_key(|g| g.cluster)
                        .map_or(line.width, LineGlyph::trailing_edge)
                },
                LineGlyph::leading_edge,
            ),
        TextAffinity::Upstream => line
            .glyphs
            .iter()
            .filter(|g| g.cluster < target)
            .max_by_key(|g| g.cluster)
            .map_or_else(
                || {
                    line.glyphs
                        .iter()
                        .min_by_key(|g| g.cluster)
                        .map_or(0.0, LineGlyph::leading_edge)
                },
                LineGlyph::trailing_edge,
            ),
    }
}

const fn order_positions(a: HitPosition, b: HitPosition) -> (HitPosition, HitPosition) {
    if a.byte_index <= b.byte_index {
        (a, b)
    } else {
        (b, a)
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
