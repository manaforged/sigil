//! Owned paragraph wrapper (DR-044, G1.4).
//!
//! A [`Paragraph`] is a list of [`LayoutLine`]s plus shared font data,
//! held behind [`Rc`] so cache hits return a cheap clone. The Parley
//! `Layout<Color>` substrate is gone — every method below operates on
//! the owned glyph runs produced by the engine.

#![allow(
    clippy::redundant_pub_crate,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]

use std::cell::Cell;
use std::rc::Rc;
use std::sync::Arc;

use sigil_hal::{Color, HitPosition, ParagraphMetrics, Point, Rect, RenderedGlyph, TextAffinity};
use smallvec::SmallVec;

use crate::Font;
use crate::glyph::GlyphRun;

/// Single laid-out line.
///
/// `LayoutLine` is mutable while the engine assembles glyphs into it;
/// once finalised (via [`set_baseline_to`](Self::set_baseline_to)) it
/// is treated as read-only by all consumers.
#[derive(Debug, Clone)]
pub(crate) struct LayoutLine {
    /// Glyphs in visual order (left-to-right in this G1.4 build).
    glyphs: SmallVec<[LineGlyph; 32]>,
    /// Index of this line within the paragraph (0-based).
    line_index: u32,
    /// Accumulated horizontal advance.
    width: f32,
    /// Y of the baseline (top of paragraph = 0).
    baseline_y: f32,
    /// Vertical extent of the line (line-height).
    height: f32,
    /// Byte range covered by this line, `[start, end)`.
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

    /// Stamp the true logical byte position of a line that has NO
    /// segments (consecutive hard breaks, or a leading/trailing hard
    /// break with nothing on this line) — `byte` is the offset of the
    /// hard break that created it (or `0` for a fully empty
    /// paragraph), not the `u32::MAX..0` sentinel [`Self::empty`]
    /// starts with. Hit-testing and caret placement on an empty line
    /// depend on this: without it every empty line looks identical
    /// (byte 0), so a click on the SECOND blank line of `"a\n\nb"`
    /// would incorrectly move the caret to the document start (the reviewer
    /// adversarial review, re-gate finding B).
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

    /// Stamp the baseline at `baseline_y` measured from the top of
    /// the paragraph, from the paragraph strut (`ascent`/`descent`
    /// both positive, px).
    ///
    /// CSS half-leading model: the strut's ascent+descent block is
    /// centred in the line box, so the baseline sits
    /// `(lh − (ascent + descent)) / 2 + ascent` below the line top.
    /// A degenerate strut (broken face metrics) falls back to an
    /// 80%-of-line-box baseline.
    pub(crate) fn set_baseline_to(&mut self, line_height: f32, ascent: f32, descent: f32) {
        let lh = line_height.max(1.0);
        self.height = lh;
        let offset = if ascent > 0.0 {
            (lh - (ascent + descent)).mul_add(0.5, ascent)
        } else {
            lh * 0.8
        };
        // Precision loss on line_index → f32 is fine: line counts > 2^23
        // are unreachable in practice for a single paragraph.
        #[allow(clippy::cast_precision_loss)]
        let idx = self.line_index as f32;
        self.baseline_y = idx.mul_add(lh, offset);
    }

    fn line_top(&self) -> f32 {
        // Lines are uniform-height boxes stacked from the paragraph
        // top — independent of where the baseline falls inside them.
        #[allow(clippy::cast_precision_loss)]
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

/// A single positioned glyph stored on a line.
#[derive(Debug, Clone, Copy)]
struct LineGlyph {
    glyph_id: u32,
    /// X offset of glyph origin from the line's left edge.
    x: f32,
    advance: f32,
    offset_y: f32,
    /// UTF-8 byte index in the source string.
    cluster: u32,
    /// UAX #9 bidi embedding level — even is LTR, odd is RTL. Drives
    /// direction-aware caret placement, hit-testing, and per-visual-run
    /// selection geometry.
    level: u8,
    /// Index into the paragraph's [`FaceEntry`] table — which face
    /// this glyph was shaped with (Wave 4 per-run font fallback).
    face_slot: u16,
}

impl LineGlyph {
    /// `true` when this glyph belongs to a right-to-left bidi run.
    const fn is_rtl(&self) -> bool {
        self.level & 1 == 1
    }

    /// Visual x of the glyph's logical *leading* edge — where a caret
    /// sits when placed *before* this glyph in logical order. The left
    /// edge for LTR, the right edge for RTL.
    fn leading_edge(&self) -> f32 {
        if self.is_rtl() {
            self.x + self.advance
        } else {
            self.x
        }
    }

    /// Visual x of the glyph's logical *trailing* edge — where a caret
    /// sits when placed *after* this glyph in logical order.
    fn trailing_edge(&self) -> f32 {
        if self.is_rtl() {
            self.x
        } else {
            self.x + self.advance
        }
    }
}

/// Arguments for [`LayoutLine::append_segment`] — one shaped run
/// (uniform bidi level + script + face) being appended to a line.
pub(crate) struct SegmentAppend<'a> {
    pub(crate) run: &'a GlyphRun,
    pub(crate) advance: f32,
    pub(crate) byte_start: u32,
    pub(crate) byte_end: u32,
    /// UAX #9 bidi embedding level of this run (even LTR, odd RTL).
    pub(crate) level: u8,
    /// Index into the paragraph's per-run font fallback chain.
    pub(crate) face_slot: u16,
}

/// Cached min/max content widths to avoid re-traversal.
#[derive(Clone, Copy, Default)]
struct ContentWidths {
    min: f32,
    max: f32,
}

/// Em-scale and leading for a paragraph, kept distinct.
///
/// `font_size` is the point size the outlines were shaped at — it is the
/// em→pixel scale the renderer must use for glyph ink. `line_height` is the
/// line-box height (leading) that drives baseline-to-baseline row advance.
/// Conflating the two (rasterising ink at `line_height`) oversizes every glyph
/// by the line-height factor; they are carried separately so that cannot happen.
#[derive(Clone, Copy)]
pub(crate) struct TextMetrics {
    /// Point size the glyphs were shaped at, in logical pixels (the em-scale).
    pub font_size: f32,
    /// Line-box height (leading), in logical pixels — row advance, not ink scale.
    pub line_height: f32,
}

/// One registered face inside a paragraph's per-run fallback table
/// (Wave 4). Indexed by [`LineGlyph::face_slot`] /
/// [`SegmentAppend::face_slot`].
struct FaceEntry {
    bytes: Arc<Vec<u8>>,
    index: u32,
    id: u64,
}

impl From<&Font> for FaceEntry {
    fn from(font: &Font) -> Self {
        Self {
            bytes: font.bytes_arc(),
            index: font.face_index(),
            id: font.id(),
        }
    }
}

/// Shared inner state behind `Rc`.
struct ParagraphInner {
    lines: Vec<LayoutLine>,
    /// Em-scale for glyph ink — the size outlines were shaped at.
    font_size: f32,
    line_height: f32,
    /// Per-run font fallback table — index 0 is always the paragraph's
    /// primary face; the rest are whichever other registered faces the
    /// engine tried for coverage. Never empty.
    fonts: Vec<FaceEntry>,
    /// Widest unbreakable word/segment from the engine. Drives
    /// min-content width for CSS-style intrinsic sizing.
    min_content_hint: f32,
    content_widths: Cell<Option<ContentWidths>>,
}

/// A laid-out paragraph of text (owned shaper output).
///
/// Inner state is held behind [`Rc`] so the engine cache returns cheap
/// clones; consumers should treat the paragraph as immutable. The
/// public surface matches the historical Parley-backed version
/// modulo `inner()` which is gone (`Layout<Color>` is no longer the
/// substrate).
#[derive(Clone)]
pub struct Paragraph {
    inner: Rc<ParagraphInner>,
}

impl Paragraph {
    /// `faces` is the paragraph's per-run font fallback chain — index 0
    /// is the primary face for the active style, the rest are the
    /// other faces the engine tried for coverage (Wave 4). Must not be
    /// empty.
    pub(crate) fn new(
        lines: Vec<LayoutLine>,
        metrics: TextMetrics,
        faces: &[Font],
        min_content_hint: f32,
    ) -> Self {
        let fonts: Vec<FaceEntry> = faces.iter().map(FaceEntry::from).collect();
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
        // Min-content width is the widest unbreakable word/segment,
        // populated by the engine during shape_segments. Falls back to
        // max width when the hint is missing (e.g. empty paragraph).
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

    /// Hit-test a point against the paragraph.
    ///
    /// Returns the nearest cluster-boundary position suitable for caret
    /// placement. Points outside the layout area snap to the closest
    /// edge.
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

    /// Returns the caret rectangle for a hit position.
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

    /// Calls `f` with each bounding rectangle of the selection range, one
    /// per contiguous VISUAL span.
    ///
    /// A logically-contiguous selection can map to several disjoint visual
    /// spans once bidi reordering interleaves runs (an RTL run embedded in
    /// LTR text, or the reverse). Rather than fill the gap between the two
    /// logical endpoints as one rectangle — which would paint over unrelated
    /// text between the spans — this walks the line's glyphs in visual order
    /// and emits one rectangle per maximal run of selected glyphs. Because a
    /// bidi run is contiguous in visual order, each emitted rectangle covers
    /// exactly one run's contribution to the selection.
    ///
    /// Selection granularity is whole glyphs: a glyph counts as selected when
    /// its logical cluster start falls in `[lo, hi)`. Endpoints are grapheme
    /// boundaries, so no glyph is split.
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

    /// Returns all selection highlight rectangles as a `Vec`.
    #[must_use]
    pub fn selection_rects(&self, anchor: HitPosition, focus: HitPosition) -> Vec<Rect> {
        let mut rects = Vec::new();
        self.selection_rects_with(anchor, focus, |r| rects.push(r));
        rects
    }

    /// All positioned glyphs in layout order, ready for GPU rendering.
    ///
    /// Each glyph's `font_id`/`font_index` come from the face it was
    /// actually shaped with — the paragraph's per-run fallback face
    /// when the primary lacked coverage for that cluster, the primary
    /// otherwise (Wave 4).
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
                    // Em-scale is the shaped point size — NOT line_height. Stamping
                    // line_height here rasterised every glyph 1.2× too large for the
                    // advances it was positioned with (the renderer uses this as the
                    // sole em→px scale at draw.rs `scale = g.size / upem`).
                    size: self.inner.font_size,
                    color: Color::WHITE,
                    variation_coords: Box::new([]),
                });
            }
        }
        out
    }

    /// Resolve a [`LineGlyph::face_slot`] to its [`FaceEntry`], falling
    /// back to the primary face (slot 0) on an out-of-range index —
    /// defensive only; `fonts` always has one entry per chain slot a
    /// glyph could have been stamped with.
    fn face_for_slot(&self, slot: u16) -> &FaceEntry {
        self.inner
            .fonts
            .get(slot as usize)
            .or_else(|| self.inner.fonts.first())
            .expect("Paragraph::new always seeds at least the primary face")
    }

    /// Calls `f` for each font face registered in the paragraph's
    /// per-run fallback chain — not just the ones a glyph actually
    /// used. Cheap: the chain is small (≤4 faces) and the caller
    /// (the renderer's outline-preprocess pass) only acts on the
    /// `font_id`s it actually needs.
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
    // Snap to first or last depending on whether y is above or below.
    if y < 0.0 {
        lines.first().expect("non-empty (caller guards)")
    } else {
        lines.last().expect("non-empty (caller guards)")
    }
}

/// Find the line whose byte range covers `byte_index`, for caret
/// placement. Empty lines now carry a real, distinct byte position
/// (`LayoutLine::set_empty_byte_position`, the reviewer re-gate finding B)
/// instead of a shared sentinel, so a byte that lands exactly on an
/// empty line (consecutive hard breaks) correctly matches THAT line —
/// no `!l.glyphs.is_empty()` guard needed to dodge sentinel collisions
/// anymore.
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
    // The visual-left/right edges of the line are the logical START of the
    // leftmost/rightmost VISUAL glyph only for LTR; for an RTL edge glyph
    // they are its logical END. Resolve each edge from that glyph's direction
    // so a click past a pure-RTL line's left edge lands at the reading end,
    // not the start.
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
    // The cluster start sits at the glyph's LOGICAL leading edge: the visual
    // LEFT for an LTR glyph, the visual RIGHT for an RTL one. So the half of
    // the glyph nearest the leading edge picks this cluster (downstream); the
    // other half picks the next cluster boundary (upstream). "Next" is the
    // smallest recorded cluster greater than this one, not `+1`, so a
    // multibyte codepoint (e.g. the 2-byte `é`) is not split mid-sequence.
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

/// Resolve the hit at one visual edge of glyph `g` (`visual_left` picks its
/// left edge, else its right). The glyph's leading (logical-start) edge is
/// its visual left for LTR and its visual right for RTL, so the caret is the
/// cluster start (downstream) at the leading edge and the next boundary
/// (upstream) at the trailing edge.
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

/// Byte offset of the cluster boundary immediately after `cluster` within the
/// line — i.e. the real UTF-8 width of the glyph at `cluster`, taken from the
/// shaper's recorded cluster offsets rather than guessed. Returns `byte_end`
/// when `cluster` belongs to the line's last glyph.
fn next_cluster_after(line: &LayoutLine, cluster: u32) -> u32 {
    line.glyphs
        .iter()
        .map(|glyph| glyph.cluster)
        .filter(|&candidate| candidate > cluster)
        .min()
        .unwrap_or(line.byte_end)
}

/// Resolve the physical caret x for `(byte, affinity)` on `line`, correct
/// across bidi runs.
///
/// `line.glyphs` is in VISUAL (paint) order after the Wave 4 reorder, but
/// each glyph carries its logical `cluster` and bidi `level`, so the caret
/// is placed relative to the glyph its byte + affinity picks in LOGICAL
/// order, using that glyph's direction:
///
/// - **Downstream** binds the caret to the glyph that *starts* at `byte`
///   (the next glyph logically), at that glyph's leading edge.
/// - **Upstream** binds it to the glyph that *ends* at `byte` (the previous
///   glyph logically), at that glyph's trailing edge.
///
/// A glyph's leading/trailing edge is its left/right for LTR and swapped
/// for RTL, so a caret at a run boundary resolves to two distinct visual x
/// positions per affinity, and a pure-RTL line's logical start sits at the
/// visual right edge (not `0.0`).
/// Emit one selection rectangle for a completed visual span `(left, right)`
/// on `line`, skipping empty spans.
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
                    // No glyph starts at/after `target` — it is the logical
                    // line end. Sit at the last logical glyph's trailing edge.
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
                    // Nothing before `target` — the logical line start. Sit at
                    // the first logical glyph's leading edge.
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
