use smallvec::SmallVec;
use unicode_segmentation::UnicodeSegmentation;

use super::{run_has_disallowed_notdef, run_total_advance};
use crate::text::glyph::ShapedRun;
use crate::text::shaper::{FaceStyle, Font, Shaper, TextSegment};

pub(super) struct FaceSpan {
    pub(super) start: usize,
    pub(super) end: usize,
    pub(super) face_slot: u16,
}

pub(super) fn coalesce_face_spans(chain: &[Font], fragment: &str) -> SmallVec<[FaceSpan; 4]> {
    let mut spans = SmallVec::<[FaceSpan; 4]>::new();
    if chain
        .first()
        .is_some_and(|primary| primary.covers(fragment))
    {
        spans.push(FaceSpan {
            start: 0,
            end: fragment.len(),
            face_slot: 0,
        });
        return spans;
    }
    for (offset, cluster) in fragment.grapheme_indices(true) {
        let (face_slot, _) = select_face(chain, cluster);
        let end = offset + cluster.len();
        if let Some(last) = spans.last_mut()
            && last.face_slot == face_slot
            && last.end == offset
        {
            last.end = end;
        } else {
            spans.push(FaceSpan {
                start: offset,
                end,
                face_slot,
            });
        }
    }
    spans
}

fn select_face<'a>(chain: &'a [Font], fragment: &str) -> (u16, &'a Font) {
    for (idx, font) in chain.iter().enumerate() {
        if font.covers(fragment) {
            return (u16::try_from(idx).unwrap_or(0), font);
        }
    }
    let primary = chain
        .first()
        .expect("fallback_chain always seeds the primary face");
    (0, primary)
}

pub(super) fn cell_width(chain: &[Font], style: &FaceStyle) -> f32 {
    chain.first().map_or(style.size_px, |primary| {
        let sample = Shaper::new(primary).shape(&TextSegment::at_level("0", 0), style);
        run_total_advance(&sample)
    })
}

const WIDE_MARGIN: f32 = 0.3;

pub(super) fn fit_cells(run: &mut ShapedRun, cell: f32) {
    if cell <= 0.0 {
        return;
    }
    for group in run
        .glyphs
        .chunk_by_mut(|left, right| left.cluster == right.cluster)
    {
        let natural: f32 = group.iter().map(|glyph| glyph.advance).sum();
        let cells = (natural / cell + WIDE_MARGIN).floor().max(1.0);
        let extra = cells.mul_add(cell, -natural);
        if let Some(last) = group.last_mut() {
            last.advance += extra;
        }
        if let Some(first) = group.first_mut() {
            first.offset_x += extra / 2.0;
        }
        run.width += extra;
    }
}

pub(super) fn shape_in_chain(
    chain: &[Font],
    initial_slot: u16,
    fragment: &str,
    level: u8,
    style: &FaceStyle,
) -> (u16, ShapedRun) {
    let seg = TextSegment::at_level(fragment, level);
    if let Some(font) = chain.get(initial_slot as usize) {
        let run = Shaper::new(font).shape(&seg, style);
        if !run_has_disallowed_notdef(&run, fragment) {
            return (initial_slot, run);
        }
        for (idx, candidate) in chain.iter().enumerate() {
            let slot = u16::try_from(idx).unwrap_or(0);
            if slot == initial_slot {
                continue;
            }
            let candidate_run = Shaper::new(candidate).shape(&seg, style);
            if !run_has_disallowed_notdef(&candidate_run, fragment) {
                return (slot, candidate_run);
            }
        }
        return (initial_slot, run);
    }
    let primary = chain
        .first()
        .expect("fallback_chain always seeds the primary face");
    (0, Shaper::new(primary).shape(&seg, style))
}
