use std::ops::Range;
use std::sync::Arc;

use smallvec::SmallVec;

use unicode_segmentation::UnicodeSegmentation;

use crate::bidi;
use crate::style::{TextFace, TextRuns, TextStyle};
use crate::text::FaceSpec;
use crate::text::font_settings::{Axes, FontSettings};
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

struct Piece<'a> {
    range: Range<usize>,
    family: &'a str,
    weight: u16,
    italic: bool,
    font: FontSettings,
}

impl<'a> Piece<'a> {
    fn base(range: Range<usize>, style: &'a TextStyle) -> Self {
        Self {
            range,
            family: style.font_family.as_str(),
            weight: style.font_weight,
            italic: style.italic,
            font: FontSettings::new(),
        }
    }

    fn run(range: Range<usize>, face: &'a TextFace, style: &'a TextStyle) -> Self {
        Self {
            range,
            family: TextRuns::family(face, &style.font_family),
            weight: TextRuns::weight(face, style.font_weight),
            italic: style.italic || face.italic,
            font: face.font_settings,
        }
    }
}

fn pieces<'a>(text: &str, style: &'a TextStyle) -> SmallVec<[Piece<'a>; 4]> {
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
            out.push(Piece::base(at..start, style));
        }
        out.push(Piece::run(start..end, &run.face, style));
        at = end;
    }
    if at < text.len() || out.is_empty() {
        out.push(Piece::base(at..text.len(), style));
    }
    out
}

struct Chain<'a> {
    family: &'a str,
    weight: u16,
    italic: bool,
    axes: Axes,
    slot: u16,
    fonts: Arc<[Font]>,
}

#[derive(Default)]
struct Faces<'a> {
    chains: SmallVec<[Chain<'a>; 2]>,
    specs: SmallVec<[FaceSpec; 8]>,
}

impl<'a> Faces<'a> {
    fn chain(
        &mut self,
        fonts: &FontCollection,
        (piece, axes): (&Piece<'a>, Axes),
        chain_of: &mut impl FnMut(&Font, u16) -> Arc<[Font]>,
    ) -> (u16, Arc<[Font]>) {
        let known = self.chains.iter().find(|chain| {
            chain.family == piece.family
                && chain.weight == piece.weight
                && chain.italic == piece.italic
                && chain.axes == axes
        });
        if let Some(chain) = known {
            return (chain.slot, chain.fonts.clone());
        }
        let (primary, skew) = fonts.select_face(piece.family, piece.weight, piece.italic);
        let chain = chain_of(primary, piece.weight);
        let slot = u16::try_from(self.specs.len()).unwrap_or(u16::MAX);
        self.specs.extend(chain.iter().map(|face| FaceSpec {
            font: face.clone(),
            weight: piece.weight,
            skew,
            axes,
        }));
        self.chains.push(Chain {
            family: piece.family,
            weight: piece.weight,
            italic: piece.italic,
            axes,
            slot,
            fonts: chain.clone(),
        });
        (slot, chain)
    }
}

pub(super) fn style_spans(
    fonts: &FontCollection,
    text: &str,
    style: &TextStyle,
    shaper: &FaceStyle,
    chain_of: &mut impl FnMut(&Font, u16) -> Arc<[Font]>,
) -> (SmallVec<[ShapeSpan; 2]>, SmallVec<[FaceSpec; 8]>) {
    let mut faces = Faces::default();
    let mut spans = SmallVec::<[ShapeSpan; 2]>::new();
    for piece in pieces(text, style) {
        let font = piece.font.over(shaper.font);
        let (slot, chain) = faces.chain(fonts, (&piece, font.axes), chain_of);
        let face_style = FaceStyle {
            weight: piece.weight,
            font,
            ..shaper.clone()
        };
        let key = SpanKey::new(&chain, &face_style);
        spans.push(ShapeSpan {
            start: piece.range.start,
            end: piece.range.end,
            chain,
            style: face_style,
            key,
            slot,
        });
    }
    (spans, faces.specs)
}
