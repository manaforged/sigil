use std::ops::Range;
use std::sync::Arc;

use smallvec::SmallVec;

use unicode_segmentation::UnicodeSegmentation;

use crate::bidi;
use crate::style::{TextRuns, TextStyle};
use crate::text::FaceSpec;
use crate::text::glyph::ShapedRun;
use crate::text::shaper::{FaceStyle, Font, Script};

use super::FontCollection;
use super::shape::{
    ShapedSegment, SpanKey, WordCtx, is_line_terminator, mark_break_opportunities,
    split_word_into_uniform_runs, u32_clamp, with_words,
};

pub(super) struct ShapeSpan {
    pub(super) start: usize,
    pub(super) end: usize,
    pub(super) chain: Arc<[Font]>,
    pub(super) style: FaceStyle,
    pub(super) key: SpanKey,
    pub(super) slot: u16,
}

#[cfg(feature = "kittest")]
pub(super) fn shape_segments(
    chain: &[Font],
    text: &str,
    style: &FaceStyle,
    paragraph_bidi: &bidi::ParagraphBidi<'_>,
) -> Vec<ShapedSegment> {
    let key = SpanKey::new(chain, style);
    let span = ShapeSpan {
        start: 0,
        end: text.len(),
        chain: Arc::from(chain),
        style: style.clone(),
        key,
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
    let mut out: Vec<ShapedSegment> = Vec::with_capacity(text.len() / 2 + 1);
    let mut first_span = 0;
    with_words(|words| {
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
            for span in overlapping(spans, &mut first_span, word_start..word_end) {
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
                    span_key: &span.key,
                };
                split_word_into_uniform_runs(&ctx, &levels, words, &mut out);
                for seg in out.iter_mut().skip(first) {
                    seg.face_slot = seg.face_slot.saturating_add(span.slot);
                }
            }
        }
    });
    mark_break_opportunities(text, &mut out);
    out
}

fn overlapping<'s>(
    spans: &'s [ShapeSpan],
    first: &mut usize,
    word: Range<usize>,
) -> &'s [ShapeSpan] {
    while spans.get(*first).is_some_and(|span| span.end <= word.start) {
        *first += 1;
    }
    let rest = spans.get(*first..).unwrap_or_default();
    let count = rest.iter().take_while(|span| span.start < word.end).count();
    rest.get(..count).unwrap_or_default()
}

fn hard_break(start: usize, len: usize) -> ShapedSegment {
    ShapedSegment {
        start: u32_clamp(start),
        end: u32_clamp(start + len),
        run: ShapedRun::empty(),
        advance: 0.0,
        is_break_opportunity: true,
        breakable_before: false,
        is_hard_break: true,
        level: 0,
        #[cfg(feature = "kittest")]
        script: Script::Common,
        face_slot: 0,
    }
}

type Piece<'a> = (usize, usize, &'a str, u16, bool);

fn pieces<'a>(text: &str, style: &'a TextStyle) -> SmallVec<[Piece<'a>; 4]> {
    let base = (style.font_family.as_str(), style.font_weight, style.italic);
    let mut out = SmallVec::new();
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

type Chain<'a> = (&'a str, u16, bool, u16, Arc<[Font]>);

pub(super) fn style_spans(
    fonts: &FontCollection,
    text: &str,
    style: &TextStyle,
    shaper: &FaceStyle,
    chain_of: &mut impl FnMut(&Font, u16) -> Arc<[Font]>,
) -> (SmallVec<[ShapeSpan; 2]>, SmallVec<[FaceSpec; 8]>) {
    let mut chains = SmallVec::<[Chain<'_>; 2]>::new();
    let mut faces = SmallVec::<[FaceSpec; 8]>::new();
    let mut spans = SmallVec::<[ShapeSpan; 2]>::new();
    for (start, end, family, weight, italic) in pieces(text, style) {
        let known = chains
            .iter()
            .find(|entry| entry.0 == family && entry.1 == weight && entry.2 == italic)
            .map(|entry| (entry.3, entry.4.clone()));
        let (slot, chain) = known.unwrap_or_else(|| {
            let (primary, skew) = fonts.select_face(family, weight, italic);
            let chain = chain_of(primary, weight);
            let slot = u16::try_from(faces.len()).unwrap_or(u16::MAX);
            faces.extend(chain.iter().map(|font| FaceSpec {
                font: font.clone(),
                weight,
                skew,
            }));
            chains.push((family, weight, italic, slot, chain.clone()));
            (slot, chain)
        });
        let face_style = FaceStyle {
            weight,
            ..shaper.clone()
        };
        let key = SpanKey::new(&chain, &face_style);
        spans.push(ShapeSpan {
            start,
            end,
            chain,
            style: face_style,
            key,
            slot,
        });
    }
    (spans, faces)
}
