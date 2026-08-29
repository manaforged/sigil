use std::cell::{Cell, Ref, RefCell};
use std::rc::Rc;

use sigil_hal::{ParagraphMetrics, TextEngine};
use sigil_style::{StyledTextEngine, TextLineHeight, TextStyle};
use unicode_segmentation::UnicodeSegmentation;

use crate::bidi;
use crate::cache::ParagraphCache;
use crate::config::TextConfig;
use crate::error::TextError;
use crate::glyph::GlyphRun;
use crate::paragraph::{LayoutLine, Paragraph, SegmentAppend, TextMetrics};
use crate::shaper::{Font, Script, Shaper, Style, TextSegment};

pub struct OwnedTextEngine {
    fonts: RefCell<FontCollection>,
    font_gen: Cell<u64>,
    config: TextConfig,
    cache: RefCell<ParagraphCache>,
}

impl OwnedTextEngine {
    #[must_use]
    pub fn new(config: TextConfig) -> Self {
        Self {
            fonts: RefCell::new(FontCollection::with_default()),
            font_gen: Cell::new(crate::fonts::default_fonts_generation()),
            config,
            cache: RefCell::new(ParagraphCache::new()),
        }
    }

    #[must_use]
    pub fn thread() -> Rc<Self> {
        thread_local! {
            static ENGINE: Rc<OwnedTextEngine> =
                Rc::new(OwnedTextEngine::new(TextConfig::default()));
        }
        ENGINE.with(Rc::clone)
    }

    fn refresh_fonts(&self) {
        let generation = crate::fonts::default_fonts_generation();
        if generation != self.font_gen.get() {
            *self.fonts.borrow_mut() = FontCollection::with_default();
            self.font_gen.set(generation);
            self.cache.borrow_mut().clear();
        }
    }

    pub fn clear_cache(&self) {
        self.cache.borrow_mut().clear();
    }

    pub fn layout_paragraph_styled(
        &self,
        text: &str,
        max_width: Option<f32>,
        style: &TextStyle,
    ) -> Result<Paragraph, TextError> {
        self.refresh_fonts();
        if let Some(cached) = self.cache.borrow().get(text, max_width, style) {
            return Ok(cached.clone());
        }
        let para = self.build_paragraph(text, max_width, style);
        self.cache
            .borrow_mut()
            .insert(text, max_width, style, para.clone());
        Ok(para)
    }

    pub fn layout_paragraph_ellipsized(
        &self,
        text: &str,
        max_width: f32,
        style: &TextStyle,
    ) -> Result<Paragraph, TextError> {
        const ELLIPSIS: &str = "\u{2026}";
        let full = self.layout_paragraph_styled(text, None, style)?;
        if full.width() <= max_width {
            return Ok(full);
        }
        let ellipsis = self.layout_paragraph_styled(ELLIPSIS, None, style)?;
        if ellipsis.width() >= max_width {
            return Ok(ellipsis);
        }
        let budget = max_width - ellipsis.width();

        let bounds: Vec<usize> = text
            .char_indices()
            .map(|(i, _)| i)
            .chain(std::iter::once(text.len()))
            .collect();
        let bound_at = |k: usize| bounds.get(k).copied().unwrap_or(text.len());
        let (mut lo, mut hi) = (0usize, bounds.len() - 1);
        while lo < hi {
            let mid = lo + (hi - lo).div_ceil(2);
            let prefix = text.get(..bound_at(mid)).unwrap_or("");
            let fits = self
                .layout_paragraph_styled(prefix, None, style)
                .is_ok_and(|p| p.width() <= budget);
            if fits {
                lo = mid;
            } else {
                hi = mid - 1;
            }
        }
        let kept = text.get(..bound_at(lo)).unwrap_or("").trim_end();
        let mut out = String::with_capacity(kept.len() + ELLIPSIS.len());
        out.push_str(kept);
        out.push_str(ELLIPSIS);
        self.layout_paragraph_styled(&out, None, style)
    }

    #[must_use]
    pub fn fonts(&self) -> Ref<'_, FontCollection> {
        self.fonts.borrow()
    }

    #[doc(hidden)]
    #[cfg(any(test, debug_assertions))]
    #[must_use]
    pub fn debug_segments_for_tests(&self, text: &str) -> Vec<(u32, u32, u8, &'static str)> {
        let fonts = self.fonts.borrow();
        let primary = fonts.select_weight("sans", self.config.font_weight);
        let chain = fonts.fallback_chain(primary, self.config.font_weight);
        let style = to_shaper_style(&self.config.text_style());
        let paragraph_bidi = bidi::ParagraphBidi::new(text);
        shape_segments(&chain, text, &style, &paragraph_bidi)
            .into_iter()
            .map(|s| (s.start, s.end, s.level, script_name(s.script)))
            .collect()
    }

    pub fn register_font(&mut self, bytes: Vec<u8>, face_index: u32) -> Result<(), TextError> {
        self.fonts.borrow_mut().register(bytes, face_index)?;
        self.cache.borrow_mut().clear();
        Ok(())
    }

    pub fn register_font_family(
        &mut self,
        family: &str,
        bytes: Vec<u8>,
        face_index: u32,
    ) -> Result<(), TextError> {
        self.fonts
            .borrow_mut()
            .register_family(family, bytes, face_index)?;
        self.cache.borrow_mut().clear();
        Ok(())
    }

    fn build_paragraph(&self, text: &str, max_width: Option<f32>, style: &TextStyle) -> Paragraph {
        let fonts = self.fonts.borrow();
        let primary = fonts.select_weight(&style.font_family, style.font_weight);
        let chain = fonts.fallback_chain(primary, style.font_weight);
        let shaper = Shaper::new(primary);
        let shaper_style = to_shaper_style(style);
        let strut = LineStrut::from_shaper(&shaper, &shaper_style, resolve_line_height(style));
        let paragraph_bidi = bidi::ParagraphBidi::new(text);
        let runs = shape_segments(&chain, text, &shaper_style, &paragraph_bidi);
        let min_content_hint = runs
            .iter()
            .filter(|s| !s.is_break_opportunity)
            .map(|s| s.advance)
            .fold(0.0_f32, f32::max);
        let lines = break_lines(&runs, max_width, strut, &paragraph_bidi);
        Paragraph::new(
            lines,
            TextMetrics {
                font_size: shaper_style.size_px,
                line_height: strut.line_height,
            },
            &chain,
            style.font_weight,
            min_content_hint,
        )
    }
}

impl TextEngine for OwnedTextEngine {
    type Error = TextError;
    type Paragraph = Paragraph;

    fn layout_paragraph(&self, text: &str, max_width: Option<f32>) -> Result<Paragraph, TextError> {
        let style = self.config.text_style();
        self.layout_paragraph_styled(text, max_width, &style)
    }

    fn clear_cache(&self) {
        self.cache.borrow_mut().clear();
    }
}

impl StyledTextEngine for OwnedTextEngine {
    fn layout_paragraph_styled(
        &self,
        text: &str,
        max_width: Option<f32>,
        style: &TextStyle,
    ) -> Result<Paragraph, TextError> {
        Self::layout_paragraph_styled(self, text, max_width, style)
    }
}

fn to_shaper_style(style: &TextStyle) -> Style {
    Style {
        size_px: style.font_size.max(1.0),
        weight: style.font_weight,
        features: smallvec::SmallVec::new(),
        letter_spacing_em: style.letter_spacing_em,
    }
}

fn resolve_line_height(style: &TextStyle) -> f32 {
    match style.line_height {
        TextLineHeight::FontSizeRelative(factor) => (style.font_size * factor.max(0.1)).max(1.0),
        TextLineHeight::Absolute(value) => value.max(1.0),
    }
}

#[derive(Clone, Copy)]
struct LineStrut {
    line_height: f32,
    ascent: f32,
    descent: f32,
}

impl LineStrut {
    fn from_shaper(shaper: &Shaper<'_>, style: &Style, line_height: f32) -> Self {
        let (ascent, descent) = shaper.strut(style);
        Self {
            line_height,
            ascent,
            descent,
        }
    }
}

struct ShapedSegment {
    start: u32,
    end: u32,
    run: GlyphRun,
    advance: f32,
    is_break_opportunity: bool,
    breakable_before: bool,
    is_hard_break: bool,
    level: u8,
    #[allow(dead_code, reason = "intentional, reviewed trade-off")]
    script: Script,
    face_slot: u16,
}

fn shape_segments(
    chain: &[Font],
    text: &str,
    style: &Style,
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
            out.push(ShapedSegment {
                start: u32_clamp(word_start),
                end: u32_clamp(word_start + fragment.len()),
                run: GlyphRun::empty(),
                advance: 0.0,
                is_break_opportunity: true,
                breakable_before: false,
                is_hard_break: true,
                level: 0,
                script: Script::Common,
                face_slot: 0,
            });
            continue;
        }
        let ctx = WordCtx {
            word: fragment,
            word_start,
            is_ws: fragment.chars().all(char::is_whitespace),
            chain,
            style,
        };
        split_word_into_uniform_runs(&ctx, &levels, &mut out);
    }
    mark_break_opportunities(text, &mut out);
    out
}

fn mark_break_opportunities(text: &str, segments: &mut [ShapedSegment]) {
    let mut allowed = vec![false; text.len().saturating_add(1)];
    for (index, opportunity) in unicode_linebreak::linebreaks(text) {
        if matches!(opportunity, unicode_linebreak::BreakOpportunity::Allowed)
            && let Some(slot) = allowed.get_mut(index)
        {
            *slot = true;
        }
    }
    for seg in segments {
        seg.breakable_before = allowed.get(seg.start as usize).copied().unwrap_or(false);
    }
}

fn is_line_terminator(fragment: &str) -> bool {
    matches!(fragment, "\n" | "\r\n" | "\r" | "\u{2028}" | "\u{2029}")
}

struct WordCtx<'a> {
    word: &'a str,
    word_start: usize,
    is_ws: bool,
    chain: &'a [Font],
    style: &'a Style,
}

fn split_word_into_uniform_runs(ctx: &WordCtx<'_>, levels: &[u8], out: &mut Vec<ShapedSegment>) {
    let mut acc = RunAccumulator::new();

    for (ch_offset, ch) in ctx.word.char_indices() {
        let level = byte_level(levels, ctx.word_start + ch_offset);
        let script = char_script(ch);
        if acc.would_split(level, script) {
            acc.flush_to(ctx, ch_offset, out);
            acc.start(ch_offset, level, script);
        } else {
            acc.extend(ch_offset, level, script);
        }
    }

    acc.flush_to(ctx, ctx.word.len(), out);
}

struct RunAccumulator {
    start: usize,
    level: Option<u8>,
    script: Option<Script>,
}

impl RunAccumulator {
    const fn new() -> Self {
        Self {
            start: 0,
            level: None,
            script: None,
        }
    }

    fn would_split(&self, new_level: u8, new_script: Script) -> bool {
        match (self.level, self.script) {
            (Some(cur_level), Some(cur_script)) => {
                !same_run(cur_level, cur_script, new_level, new_script)
            }
            _ => false,
        }
    }

    const fn start(&mut self, offset: usize, level: u8, script: Script) {
        self.start = offset;
        self.level = Some(level);
        self.script = Some(script);
    }

    const fn extend(&mut self, offset: usize, level: u8, script: Script) {
        if self.level.is_none() {
            self.start(offset, level, script);
            return;
        }
        if let Some(cur) = self.script
            && matches!(cur, Script::Common | Script::Other)
            && !matches!(script, Script::Common | Script::Other)
        {
            self.script = Some(script);
        }
    }

    fn flush_to(&self, ctx: &WordCtx<'_>, end_offset: usize, out: &mut Vec<ShapedSegment>) {
        if self.start >= end_offset {
            return;
        }
        let args = PushRunArgs {
            fragment: &ctx.word[self.start..end_offset],
            start_byte: ctx.word_start + self.start,
            level: self.level.unwrap_or(0),
            script: self.script.unwrap_or(Script::Common),
            is_break_opportunity: ctx.is_ws,
        };
        push_run(&args, ctx.chain, ctx.style, out);
    }
}

fn same_run(cur_level: u8, cur_script: Script, new_level: u8, new_script: Script) -> bool {
    if cur_level != new_level {
        return false;
    }
    matches!(new_script, Script::Common | Script::Other)
        || matches!(cur_script, Script::Common | Script::Other)
        || cur_script == new_script
}

#[derive(Clone, Copy)]
struct PushRunArgs<'a> {
    fragment: &'a str,
    start_byte: usize,
    level: u8,
    script: Script,
    is_break_opportunity: bool,
}

fn push_run(args: &PushRunArgs<'_>, chain: &[Font], style: &Style, out: &mut Vec<ShapedSegment>) {
    if args.fragment.is_empty() {
        return;
    }
    for span in coalesce_face_spans(chain, args.fragment) {
        push_face_span(args, chain, style, &span, out);
    }
}

struct FaceSpan {
    start: usize,
    end: usize,
    face_slot: u16,
}

fn coalesce_face_spans(chain: &[Font], fragment: &str) -> Vec<FaceSpan> {
    let mut spans: Vec<FaceSpan> = Vec::new();
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

fn push_face_span(
    args: &PushRunArgs<'_>,
    chain: &[Font],
    style: &Style,
    span: &FaceSpan,
    out: &mut Vec<ShapedSegment>,
) {
    let Some(sub_fragment) = args.fragment.get(span.start..span.end) else {
        return;
    };
    if sub_fragment.is_empty() {
        return;
    }
    let start_byte = args.start_byte + span.start;
    let end_byte = args.start_byte + span.end;
    let (face_slot, mut run) =
        shape_validated(chain, span.face_slot, sub_fragment, args.level, style);
    apply_letter_spacing(&mut run, style);
    let advance = run_total_advance(&run);
    out.push(ShapedSegment {
        start: u32_clamp(start_byte),
        end: u32_clamp(end_byte),
        run,
        advance,
        is_break_opportunity: args.is_break_opportunity,
        breakable_before: false,
        is_hard_break: false,
        level: args.level,
        script: args.script,
        face_slot,
    });
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

fn shape_validated(
    chain: &[Font],
    initial_slot: u16,
    fragment: &str,
    level: u8,
    style: &Style,
) -> (u16, GlyphRun) {
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

fn run_has_disallowed_notdef(run: &GlyphRun, fragment: &str) -> bool {
    run.glyphs.iter().any(|g| {
        if g.id.0 != 0 {
            return false;
        }
        let Some(rest) = fragment.get(g.cluster as usize..) else {
            return true;
        };
        let grapheme_len = rest
            .grapheme_indices(true)
            .next()
            .map_or(rest.len(), |(_, cluster)| cluster.len());
        let grapheme = rest.get(..grapheme_len).unwrap_or(rest);
        !grapheme.chars().any(crate::shaper::is_default_ignorable)
    })
}

fn byte_level(levels: &[u8], byte: usize) -> u8 {
    levels.get(byte).copied().unwrap_or(0)
}

fn char_script(ch: char) -> Script {
    use unicode_script::{Script as UScript, UnicodeScript};
    match ch.script() {
        UScript::Common | UScript::Inherited | UScript::Unknown => Script::Common,
        UScript::Latin | UScript::Greek | UScript::Cyrillic => Script::Latin,
        UScript::Han
        | UScript::Hiragana
        | UScript::Katakana
        | UScript::Hangul
        | UScript::Bopomofo => Script::Cjk,
        UScript::Arabic => Script::Arabic,
        UScript::Hebrew => Script::Hebrew,
        UScript::Devanagari => Script::Devanagari,
        UScript::Khmer => Script::Khmer,
        UScript::Thai => Script::Thai,
        UScript::Lao => Script::Lao,
        _ => Script::Other,
    }
}

#[cfg(any(test, debug_assertions))]
const fn script_name(script: Script) -> &'static str {
    match script {
        Script::Latin => "Latin",
        Script::Cjk => "Cjk",
        Script::Common => "Common",
        Script::Arabic => "Arabic",
        Script::Hebrew => "Hebrew",
        Script::Devanagari => "Devanagari",
        Script::Khmer => "Khmer",
        Script::Thai => "Thai",
        Script::Lao => "Lao",
        Script::Other => "Other",
    }
}

fn apply_letter_spacing(run: &mut GlyphRun, style: &Style) {
    let extra = style.letter_spacing_em * style.size_px;
    if extra.abs() < f32::EPSILON {
        return;
    }
    for glyph in &mut run.glyphs {
        glyph.advance += extra;
    }
}

fn run_total_advance(run: &GlyphRun) -> f32 {
    run.glyphs.iter().map(|g| g.advance).sum()
}

#[allow(
    clippy::cast_possible_truncation,
    reason = "values are bounded to the target range before the cast"
)]
const fn u32_clamp(v: usize) -> u32 {
    if v > u32::MAX as usize {
        u32::MAX
    } else {
        v as u32
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

fn break_lines(
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

#[cfg(test)]
#[path = "engine/tests.rs"]
mod tests;

mod collection;
pub use collection::FontCollection;
