use crate::bidi;
use crate::text::paragraph::{LayoutLine, SegmentAppend};
use crate::text::shaper::{Shaper, Style};

use super::shape::ShapedSegment;

#[derive(Clone, Copy)]
pub(super) struct LineStrut {
    pub(super) line_height: f32,
    ascent: f32,
    descent: f32,
}

impl LineStrut {
    pub(super) fn from_shaper(shaper: &Shaper<'_>, style: &Style, line_height: f32) -> Self {
        let (ascent, descent) = shaper.strut(style);
        Self {
            line_height,
            ascent,
            descent,
        }
    }
}

struct PendingLine<'a> {
    segs: Vec<&'a ShapedSegment>,
    width: f32,
}

impl<'a> PendingLine<'a> {
    const fn new() -> Self {
        Self {
            segs: Vec::new(),
            width: 0.0,
        }
    }

    const fn is_empty(&self) -> bool {
        self.segs.is_empty()
    }

    fn push(&mut self, seg: &'a ShapedSegment) {
        self.width += seg.advance;
        self.segs.push(seg);
    }
}

pub(super) fn break_lines(
    segments: &[ShapedSegment],
    max_width: Option<f32>,
    strut: LineStrut,
    paragraph_bidi: &bidi::ParagraphBidi<'_>,
) -> Vec<LayoutLine> {
    if segments.is_empty() {
        let mut line = LayoutLine::empty(0, strut.line_height);
        line.set_empty_byte_position(0);
        line.set_baseline_to(strut.line_height, strut.ascent, strut.descent);
        return vec![line];
    }
    let mut lines: Vec<LayoutLine> = Vec::new();
    let mut pending = PendingLine::new();
    let mut cursor: u32 = 0;
    for seg in segments {
        if seg.is_hard_break {
            push_visual_line(&mut lines, &pending, strut, paragraph_bidi, cursor);
            pending = PendingLine::new();
            cursor = seg.end;
            continue;
        }
        if should_wrap(&pending, seg, max_width) {
            push_visual_line(&mut lines, &pending, strut, paragraph_bidi, cursor);
            pending = PendingLine::new();
            cursor = seg.start;
            if seg.is_break_opportunity {
                continue;
            }
        }
        pending.push(seg);
    }
    push_visual_line(&mut lines, &pending, strut, paragraph_bidi, cursor);
    lines
}

fn should_wrap(pending: &PendingLine<'_>, seg: &ShapedSegment, max_width: Option<f32>) -> bool {
    let Some(limit) = max_width else {
        return false;
    };
    if seg.is_break_opportunity || !seg.breakable_before {
        return false;
    }
    if pending.is_empty() {
        return false;
    }
    pending.width + seg.advance > limit
}

fn push_visual_line(
    lines: &mut Vec<LayoutLine>,
    pending: &PendingLine<'_>,
    strut: LineStrut,
    paragraph_bidi: &bidi::ParagraphBidi<'_>,
    empty_line_byte: u32,
) {
    let index = u32::try_from(lines.len()).unwrap_or(u32::MAX);
    let mut line = LayoutLine::empty(index, strut.line_height);
    let (line_start, line_end) =
        if let (Some(first), Some(last)) = (pending.segs.first(), pending.segs.last()) {
            (first.start, last.end)
        } else {
            line.set_empty_byte_position(empty_line_byte);
            (empty_line_byte, empty_line_byte)
        };
    let segment_ends: Vec<u32> = pending.segs.iter().map(|s| s.end).collect();
    let order =
        paragraph_bidi.visual_order_for_line(line_start as usize, line_end as usize, &segment_ends);
    for logical_idx in order {
        let Some(seg) = pending.segs.get(logical_idx) else {
            continue;
        };
        line.append_segment(&SegmentAppend {
            run: &seg.run,
            advance: seg.advance,
            byte_start: seg.start,
            byte_end: seg.end,
            level: seg.level,
            face_slot: seg.face_slot,
        });
    }
    finalize_line(lines, line, strut);
}

fn finalize_line(out: &mut Vec<LayoutLine>, mut line: LayoutLine, strut: LineStrut) {
    line.set_baseline_to(strut.line_height, strut.ascent, strut.descent);
    out.push(line);
}
