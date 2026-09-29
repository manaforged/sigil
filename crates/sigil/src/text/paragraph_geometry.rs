use crate::geometry::{Point, Rect};
use crate::hal::{HitPosition, RenderedGlyph, TextAffinity};
use crate::style::Color;

use super::{FaceEntry, LayoutLine, LineGlyph, Paragraph, u32_clamp};

impl Paragraph {
    #[must_use]
    pub fn rendered_glyphs(&self) -> Vec<RenderedGlyph> {
        self.rendered_glyphs_in(0, usize::MAX)
    }

    #[must_use]
    pub fn rendered_glyphs_in(&self, start: usize, end: usize) -> Vec<RenderedGlyph> {
        let mut out = Vec::new();
        for line in &self.inner.lines {
            for g in &line.glyphs {
                let cluster = g.cluster as usize;
                if cluster < start || cluster >= end {
                    continue;
                }
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
                    skew: face.skew,
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
}

pub(super) fn pick_line(lines: &[LayoutLine], y: f32) -> &LayoutLine {
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

pub(super) fn locate_line_for_position(
    lines: &[LayoutLine],
    position: HitPosition,
) -> Option<&LayoutLine> {
    for (index, line) in lines.iter().enumerate() {
        if line.byte_range_contains(position.byte_index) {
            let wrapped = position.affinity == TextAffinity::Downstream
                && u32_clamp(position.byte_index) == line.byte_end
                && lines
                    .get(index + 1)
                    .is_some_and(|next| next.byte_start == line.byte_end);
            if !wrapped {
                return Some(line);
            }
        }
    }
    lines.last()
}

pub(super) fn hit_test_line(line: &LayoutLine, x: f32) -> HitPosition {
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

pub(super) fn emit_span(span: Option<(f32, f32)>, line: &LayoutLine, f: &mut impl FnMut(Rect)) {
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

pub(super) fn select_line(
    line: &LayoutLine,
    next: Option<&LayoutLine>,
    (lo, hi): (u32, u32),
    width: f32,
    f: &mut impl FnMut(Rect),
) {
    let mut span: Option<(f32, f32)> = None;
    for glyph in &line.glyphs {
        if glyph.cluster >= lo && glyph.cluster < hi {
            let (gl, gr) = (glyph.x, glyph.x + glyph.advance);
            span = Some(span.map_or((gl, gr), |(l, r)| (l.min(gl), r.max(gr))));
        } else {
            emit_span(span.take(), line, f);
        }
    }
    if let Some((bl, br)) = break_span(line, next, (lo, hi), width) {
        match span {
            Some((left, right)) if bl <= right && br >= left => {
                span = Some((left.min(bl), right.max(br)));
            }
            Some(_) | None => {
                emit_span(span.take(), line, f);
                span = Some((bl, br));
            }
        }
    }
    emit_span(span, line, f);
}

fn break_span(
    line: &LayoutLine,
    next: Option<&LayoutLine>,
    (lo, hi): (u32, u32),
    width: f32,
) -> Option<(f32, f32)> {
    if lo > line.byte_end
        || hi <= line.byte_end
        || next.is_none_or(|next| next.byte_start <= line.byte_end)
    {
        return None;
    }
    let x = caret_x_in_line(
        line,
        HitPosition {
            byte_index: line.byte_end as usize,
            affinity: TextAffinity::Upstream,
        },
    );
    let rtl = line
        .glyphs
        .iter()
        .max_by_key(|glyph| glyph.cluster)
        .is_some_and(LineGlyph::is_rtl);
    Some((
        if rtl { x - width } else { x },
        if rtl { x } else { x + width },
    ))
}

pub(super) fn caret_x_in_line(line: &LayoutLine, pos: HitPosition) -> f32 {
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

pub(super) const fn order_positions(a: HitPosition, b: HitPosition) -> (HitPosition, HitPosition) {
    if a.byte_index <= b.byte_index {
        (a, b)
    } else {
        (b, a)
    }
}
