use unicode_segmentation::UnicodeSegmentation;

use crate::style::{TextRuns, TextStyle};
use crate::text::glyph::GlyphRun;
use crate::text::shaper::{Font, Script, Style};
use crate::text::{FaceSpec, bidi};

use super::FontCollection;
use super::shape::{
    ShapedSegment, WordCtx, is_line_terminator, mark_break_opportunities,
    split_word_into_uniform_runs, u32_clamp,
};

pub(super) struct ShapeSpan {
    pub(super) start: usize,
    pub(super) end: usize,
    pub(super) chain: Vec<Font>,
    pub(super) style: Style,
    pub(super) slot: u16,
}

#[cfg(any(test, debug_assertions))]
pub(super) fn shape_segments(
    chain: &[Font],
    text: &str,
    style: &Style,
    paragraph_bidi: &bidi::ParagraphBidi<'_>,
) -> Vec<ShapedSegment> {
    let span = ShapeSpan {
        start: 0,
        end: text.len(),
        chain: chain.to_vec(),
        style: style.clone(),
        slot: 0,
    };
    shape_spans(std::slice::from_ref(&span), text, paragraph_bidi)
}

pub(super) fn shape_spans(
    spans: &[ShapeSpan],
    text: &str,
    paragraph_bidi: &bidi::ParagraphBidi<'_>,
) -> Vec<ShapedSegment> {
    if text.is_empty() {
        return Vec::new();
    }
    let levels = paragraph_bidi.levels();
    let mut out: Vec<ShapedSegment> = Vec::new();
    for (word_start, fragment) in text.split_word_bound_indices() {
        if fragment.is_empty() {
            continue;
        }
        if is_line_terminator(fragment) {
            out.push(hard_break(word_start, fragment.len()));
            continue;
        }
        let is_ws = fragment.chars().all(char::is_whitespace);
        let word_end = word_start + fragment.len();
        for span in spans
            .iter()
            .filter(|span| span.start < word_end && span.end > word_start)
        {
            let lo = span.start.max(word_start);
            let Some(word) = text.get(lo..span.end.min(word_end)) else {
                continue;
            };
            let first = out.len();
            let ctx = WordCtx {
                word,
                word_start: lo,
                is_ws,
                chain: &span.chain,
                style: &span.style,
            };
            split_word_into_uniform_runs(&ctx, &levels, &mut out);
            for seg in out.iter_mut().skip(first) {
                seg.face_slot = seg.face_slot.saturating_add(span.slot);
            }
        }
    }
    mark_break_opportunities(text, &mut out);
    out
}

fn hard_break(start: usize, len: usize) -> ShapedSegment {
    ShapedSegment {
        start: u32_clamp(start),
        end: u32_clamp(start + len),
        run: GlyphRun::empty(),
        advance: 0.0,
        is_break_opportunity: true,
        breakable_before: false,
        is_hard_break: true,
        level: 0,
        script: Script::Common,
        face_slot: 0,
    }
}

type Piece<'a> = (usize, usize, &'a str, u16, bool);

fn pieces<'a>(text: &str, style: &'a TextStyle) -> Vec<Piece<'a>> {
    let base = (style.font_family.as_str(), style.font_weight, style.italic);
    let mut out = Vec::new();
    let mut at = 0;
    for run in style.runs.iter().flat_map(|runs| runs.0.iter()) {
        let start = run.start.min(text.len());
        let end = run.end.min(text.len());
        if start < at
            || start >= end
            || !text.is_char_boundary(start)
            || !text.is_char_boundary(end)
        {
            continue;
        }
        if start > at {
            out.push((at, start, base.0, base.1, base.2));
        }
        let family = TextRuns::family(&run.face, base.0);
        let weight = TextRuns::weight(&run.face, base.1);
        out.push((start, end, family, weight, base.2 || run.face.italic));
        at = end;
    }
    if at < text.len() || out.is_empty() {
        out.push((at, text.len(), base.0, base.1, base.2));
    }
    out
}

type Chain<'a> = (&'a str, u16, bool, u16, Vec<Font>);

pub(super) fn style_spans(
    fonts: &FontCollection,
    text: &str,
    style: &TextStyle,
    shaper: &Style,
) -> (Vec<ShapeSpan>, Vec<FaceSpec>) {
    let mut chains: Vec<Chain<'_>> = Vec::new();
    let mut faces: Vec<FaceSpec> = Vec::new();
    let mut spans = Vec::new();
    for (start, end, family, weight, italic) in pieces(text, style) {
        let known = chains
            .iter()
            .find(|entry| entry.0 == family && entry.1 == weight && entry.2 == italic)
            .map(|entry| (entry.3, entry.4.clone()));
        let (slot, chain) = known.unwrap_or_else(|| {
            let (primary, skew) = fonts.select_face(family, weight, italic);
            let chain = fonts.fallback_chain(primary, weight, text);
            let slot = u16::try_from(faces.len()).unwrap_or(u16::MAX);
            faces.extend(chain.iter().map(|font| FaceSpec {
                font: font.clone(),
                weight,
                skew,
            }));
            chains.push((family, weight, italic, slot, chain.clone()));
            (slot, chain)
        });
        spans.push(ShapeSpan {
            start,
            end,
            chain,
            style: Style {
                weight,
                ..shaper.clone()
            },
            slot,
        });
    }
    (spans, faces)
}
