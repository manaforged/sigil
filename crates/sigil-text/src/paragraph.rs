//! Paragraph wrapper around `parley::Layout`.

use std::cell::Cell;
use std::rc::Rc;

use parley::layout::PositionedLayoutItem;
use parley::{Affinity, Cursor, GlyphRun, Selection};
use sigil_hal::{Color, HitPosition, ParagraphMetrics, Point, Rect, RenderedGlyph, TextAffinity};

/// Cached min/max content widths to avoid re-traversal.
#[derive(Clone, Copy, Default)]
struct ContentWidths {
    min: f32,
    max: f32,
}

/// Shared inner state behind `Rc`.
struct ParagraphInner {
    layout: parley::Layout<Color>,
    content_widths: Cell<Option<ContentWidths>>,
}

/// A laid-out paragraph of text.
///
/// Wraps a `parley::Layout<Color>` behind `Rc` so cloning is cheap (cache
/// returns shared references). The `Color` brush is set when the paragraph
/// is built; individual style spans can override it via `StyleProperty::Brush`.
#[derive(Clone)]
pub struct Paragraph {
    inner: Rc<ParagraphInner>,
}

impl Paragraph {
    pub(crate) fn new(layout: parley::Layout<Color>) -> Self {
        Self {
            inner: Rc::new(ParagraphInner {
                layout,
                content_widths: Cell::new(None),
            }),
        }
    }

    /// Borrow the underlying Parley layout.
    #[must_use]
    pub fn inner(&self) -> &parley::Layout<Color> {
        &self.inner.layout
    }

    /// Compute or return cached content widths (one traversal).
    fn content_widths(&self) -> ContentWidths {
        if let Some(cached) = self.inner.content_widths.get() {
            return cached;
        }
        let widths = self.inner.layout.calculate_content_widths();
        let result = ContentWidths {
            min: widths.min,
            max: widths.max,
        };
        self.inner.content_widths.set(Some(result));
        result
    }
}

impl Paragraph {
    /// Hit-test a point against the paragraph.
    ///
    /// Returns the nearest cluster-boundary position suitable for caret
    /// placement. Points outside the layout area snap to the closest edge.
    #[must_use]
    pub fn hit_test(&self, x: f32, y: f32) -> HitPosition {
        let cursor = Cursor::from_point(&self.inner.layout, x, y);
        HitPosition {
            byte_index: cursor.index(),
            affinity: match cursor.affinity() {
                Affinity::Downstream => TextAffinity::Downstream,
                Affinity::Upstream => TextAffinity::Upstream,
            },
        }
    }

    /// Returns the caret rectangle for a hit position.
    ///
    /// `caret_width` is the visible width in logical pixels (1.0–2.0 is
    /// typical). The rect origin is the top-left corner; height spans the
    /// line metrics.
    ///
    /// # Notes
    ///
    /// - If `pos.byte_index` is out of range the caret is placed at the
    ///   text end (silent clamp by the underlying engine).
    /// - At `byte_index == 0`, Parley forces `Downstream` affinity
    ///   regardless of `pos.affinity`. This affects bidi cursor placement
    ///   at the text start.
    #[must_use]
    pub fn caret_rect(&self, pos: HitPosition, caret_width: f32) -> Rect {
        let affinity = to_parley_affinity(pos.affinity);
        let cursor = Cursor::from_byte_index(&self.inner.layout, pos.byte_index, affinity);
        let bb = cursor.geometry(&self.inner.layout, caret_width);
        bounding_box_to_rect(bb)
    }

    /// Calls `f` with each bounding rectangle of the selection range.
    ///
    /// Zero-allocation: the callback is invoked once per highlight rectangle
    /// (typically one per wrapped line within the selection). Use this in
    /// hot rendering paths to push directly into a GPU buffer.
    ///
    /// For a collecting convenience variant use [`selection_rects`](Self::selection_rects).
    pub fn selection_rects_with(
        &self,
        anchor: HitPosition,
        focus: HitPosition,
        mut f: impl FnMut(Rect),
    ) {
        let anchor_cursor = Cursor::from_byte_index(
            &self.inner.layout,
            anchor.byte_index,
            to_parley_affinity(anchor.affinity),
        );
        let focus_cursor = Cursor::from_byte_index(
            &self.inner.layout,
            focus.byte_index,
            to_parley_affinity(focus.affinity),
        );
        let sel = Selection::new(anchor_cursor, focus_cursor);
        sel.geometry_with(&self.inner.layout, |bb, _line_idx| {
            // Line index intentionally discarded — callers distinguish lines by
            // comparing Rect::origin.y values (same y == same line).
            f(bounding_box_to_rect(bb));
        });
    }

    /// Returns all selection highlight rectangles as a `Vec`.
    ///
    /// Convenience wrapper around [`selection_rects_with`](Self::selection_rects_with).
    /// Prefer the callback variant in hot paths.
    #[must_use]
    pub fn selection_rects(&self, anchor: HitPosition, focus: HitPosition) -> Vec<Rect> {
        let mut rects = Vec::new();
        self.selection_rects_with(anchor, focus, |r| rects.push(r));
        rects
    }

    /// Returns all positioned glyphs in layout order, ready for GPU rendering.
    ///
    /// Glyph coordinates are layout-local logical pixels: `x` is the
    /// horizontal glyph origin, `y` is the baseline. Pair with
    /// [`fonts`](Self::fonts) to supply font bytes to the renderer's
    /// glyph-outline cache.
    #[must_use]
    pub fn rendered_glyphs(&self) -> Vec<RenderedGlyph> {
        let mut out = Vec::new();
        for line in self.inner.layout.lines() {
            for item in line.items() {
                let PositionedLayoutItem::GlyphRun(run) = item else {
                    continue;
                };
                push_run_glyphs(&run, &mut out);
            }
        }
        out
    }

    /// Calls `f` for each unique font face referenced in the paragraph.
    ///
    /// Arguments to `f`: `(font_id, font_bytes, font_index)`. The `font_id`
    /// matches the corresponding field in [`RenderedGlyph`]. Each `(font_id,
    /// font_index)` pair is visited at most once, so TrueType collection faces
    /// are not deduplicated when the same blob carries multiple faces.
    /// `font_bytes` is valid only for the duration of the call — copy or
    /// preprocess as needed inside `f`.
    pub fn for_each_font<F>(&self, mut f: F)
    where
        F: FnMut(u64, &[u8], u32),
    {
        // Key on (blob_id, face_index) so TTC non-first faces are not skipped.
        let mut seen: Vec<(u64, u32)> = Vec::new();
        for line in self.inner.layout.lines() {
            for item in line.items() {
                let PositionedLayoutItem::GlyphRun(run) = item else {
                    continue;
                };
                let font = run.run().font();
                let id = font.data.id();
                let idx = font.index;
                if seen.contains(&(id, idx)) {
                    continue;
                }
                seen.push((id, idx));
                // font_bytes lifetime is valid here: `font` (and `run`) are still live.
                f(id, font.data.data(), idx);
            }
        }
    }
}

/// Push `RenderedGlyph` entries for every glyph in a run.
fn push_run_glyphs(run: &GlyphRun<'_, Color>, out: &mut Vec<RenderedGlyph>) {
    let r = run.run();
    let font = r.font();
    let font_id = font.data.id();
    let font_index = font.index;
    let size = r.font_size();
    let color = run.style().brush;
    // Collect once per run; all glyphs in the run share the same variation axes.
    let variation_coords: Box<[i16]> = r.normalized_coords().iter().copied().collect();
    for glyph in run.positioned_glyphs() {
        out.push(RenderedGlyph {
            font_id,
            font_index,
            glyph_id: glyph.id,
            x: glyph.x,
            y: glyph.y,
            size,
            color,
            variation_coords: variation_coords.clone(),
        });
    }
}

/// Map our `TextAffinity` to Parley's `Affinity`.
#[inline]
const fn to_parley_affinity(a: TextAffinity) -> Affinity {
    match a {
        TextAffinity::Downstream => Affinity::Downstream,
        TextAffinity::Upstream => Affinity::Upstream,
    }
}

/// Convert a Parley `BoundingBox` (f64 corners) to our `Rect` (f32).
///
/// The f64→f32 truncation is intentional: logical-pixel coordinates
/// have sub-pixel precision well within f32 range.
///
/// `.abs()` on width and height is defensive: Parley's `BoundingBox::new`
/// stores values as-is with no normalization ("nothing forbids negative
/// width" per the Parley source). In practice `geometry_with` and
/// `cursor.geometry` never produce inverted boxes, but we guard
/// unconditionally so future Parley edge cases (RTL cursor, etc.) cannot
/// produce negative dimensions that crash a downstream renderer.
#[inline]
#[allow(clippy::cast_possible_truncation)]
fn bounding_box_to_rect(bb: parley::BoundingBox) -> Rect {
    Rect::new(
        Point::new(bb.x0 as f32, bb.y0 as f32),
        ((bb.x1 - bb.x0) as f32).abs(),
        ((bb.y1 - bb.y0) as f32).abs(),
    )
}

impl ParagraphMetrics for Paragraph {
    fn width(&self) -> f32 {
        self.inner.layout.width()
    }

    fn height(&self) -> f32 {
        self.inner.layout.height()
    }

    fn line_count(&self) -> usize {
        self.inner.layout.len()
    }

    fn min_content_width(&self) -> f32 {
        self.content_widths().min
    }

    fn max_content_width(&self) -> f32 {
        self.content_widths().max
    }
}
