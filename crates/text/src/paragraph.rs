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
    /// Index into the paragraph's [`FaceEntry`] table — which face
    /// this glyph was shaped with (Wave 4 per-run font fallback).
    face_slot: u16,
}

/// Arguments for [`LayoutLine::append_segment`] — one shaped run
/// (uniform bidi level + script + face) being appended to a line.
pub(crate) struct SegmentAppend<'a> {
    pub(crate) run: &'a GlyphRun,
    pub(crate) advance: f32,
    pub(crate) byte_start: u32,
    pub(crate) byte_end: u32,
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

    /// Calls `f` with each bounding rectangle of the selection range.
    ///
    /// ponytail: display-only bidi ceiling (the reviewer adversarial review,
    /// 2026-07-13) — this fills the interval between the two LOGICAL
    /// endpoint x positions as ONE rectangle per line. A contiguous
    /// logical selection can map to multiple disjoint VISUAL spans
    /// after bidi reordering (a selection spanning an embedded RTL run
    /// plus its LTR neighbours), which this can highlight incorrectly
    /// (unrelated text between spans, or omitted glyphs) instead of
    /// emitting one rectangle per visual run. Upgrade path: intersect
    /// the logical selection with each visual bidi run and emit one
    /// rectangle per contiguous visual span, respecting run direction
    /// and endpoint affinity. Tracked in `docs/followups.md`.
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
        for line in &self.inner.lines {
            let line_lo = line.byte_start;
            let line_hi = line.byte_end;
            let sel_lo = u32_clamp(lo.byte_index).max(line_lo);
            let sel_hi = u32_clamp(hi.byte_index).min(line_hi);
            if sel_hi <= sel_lo {
                continue;
            }
            let x_lo = caret_x_for_byte(line, sel_lo as usize);
            let x_hi = caret_x_for_byte(line, sel_hi as usize);
            let (l, r) = (x_lo.min(x_hi), x_lo.max(x_hi));
            if r - l <= 0.0 {
                continue;
            }
            f(Rect::new(
                Point::new(l, line.line_top()),
                r - l,
                line.height,
            ));
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
    if x <= 0.0 {
        return HitPosition {
            byte_index: first_cluster(line) as usize,
            affinity: TextAffinity::Downstream,
        };
    }
    if x >= line.width {
        return HitPosition {
            byte_index: line.byte_end as usize,
            affinity: TextAffinity::Upstream,
        };
    }
    let Some(best) = pick_nearest_glyph(&line.glyphs, x) else {
        return HitPosition {
            byte_index: line.byte_start as usize,
            affinity: TextAffinity::Downstream,
        };
    };
    let mid = best.advance.mul_add(0.5, best.x);
    let (byte_index, affinity) = if x < mid {
        (best.cluster as usize, TextAffinity::Downstream)
    } else {
        // "After this glyph" is the next cluster boundary, NOT cluster+1: a
        // multibyte codepoint (e.g. the 2-byte `é`) spans >1 byte, so a +1
        // floor lands mid-codepoint, which the caller then snaps back BEFORE
        // the glyph (off by a whole codepoint). The shaper records every
        // cluster offset, so the true post-glyph boundary is the smallest
        // cluster greater than this one (or the line end for the last glyph).
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

fn first_cluster(line: &LayoutLine) -> u32 {
    line.glyphs.first().map_or(line.byte_start, |g| g.cluster)
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

/// ponytail: display-only bidi ceiling (the reviewer adversarial review,
/// 2026-07-13) — discards `pos.affinity`, so at a mixed-direction
/// boundary the same byte index can't distinguish the two distinct
/// visual caret positions upstream/downstream affinity is supposed to
/// resolve. Upgrade path: retain bidi level/run-boundary metadata in
/// `LayoutLine` and resolve caret x using both byte index AND
/// affinity, with exact-edge and dual-affinity tests for pure RTL,
/// embedded RTL, and wrapped mixed lines. Tracked in
/// `docs/followups.md`.
fn caret_x_in_line(line: &LayoutLine, pos: HitPosition) -> f32 {
    caret_x_for_byte(line, pos.byte_index)
}

/// Resolve the physical x for logical byte `byte` on `line`.
///
/// `line.glyphs` is stored in VISUAL (paint) order since the Wave 4
/// bidi reorder — a plain "first glyph in iteration order with
/// `cluster >= target`" search (the pre-reorder implementation) picks
/// the wrong glyph whenever a line has more than one bidi run, because
/// visual-iteration order no longer tracks ascending cluster order.
/// The exact-match / order-independent-nearest search below is
/// correct regardless of how many runs or directions the line mixes.
///
/// ponytail: the two fast paths (`target <= line.byte_start` -> 0.0,
/// `target >= line.byte_end` -> `line.width`) still assume the
/// logical line start/end is always the visual left/right edge —
/// true for an LTR-base line, false for a pure-RTL-base line (whose
/// logical start renders at the visual RIGHT edge). See
/// [`caret_x_in_line`]'s ceiling note; same upgrade path.
fn caret_x_for_byte(line: &LayoutLine, byte: usize) -> f32 {
    let target = u32_clamp(byte);
    if target <= line.byte_start {
        return 0.0;
    }
    if target >= line.byte_end {
        return line.width;
    }
    if let Some(g) = line.glyphs.iter().find(|g| g.cluster == target) {
        return g.x;
    }
    line.glyphs
        .iter()
        .filter(|g| g.cluster >= target)
        .min_by_key(|g| g.cluster)
        .map_or(line.width, |g| g.x)
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
