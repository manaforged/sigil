use unicode_segmentation::UnicodeSegmentation;

use crate::text::glyph::GlyphRun;
use crate::text::shaper::{Font, Script, Shaper, Style, TextSegment};

pub(super) struct ShapedSegment {
    pub(super) start: u32,
    pub(super) end: u32,
    pub(super) run: GlyphRun,
    pub(super) advance: f32,
    pub(super) is_break_opportunity: bool,
    pub(super) breakable_before: bool,
    pub(super) is_hard_break: bool,
    pub(super) level: u8,
    #[allow(dead_code, reason = "intentional, reviewed trade-off")]
    pub(super) script: Script,
    pub(super) face_slot: u16,
}

pub(super) fn mark_break_opportunities(text: &str, segments: &mut [ShapedSegment]) {
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

pub(super) fn is_line_terminator(fragment: &str) -> bool {
    matches!(fragment, "\n" | "\r\n" | "\r" | "\u{2028}" | "\u{2029}")
}

pub(super) struct WordCtx<'a> {
    pub(super) word: &'a str,
    pub(super) word_start: usize,
    pub(super) is_ws: bool,
    pub(super) chain: &'a [Font],
    pub(super) style: &'a Style,
}

pub(super) fn split_word_into_uniform_runs(
    ctx: &WordCtx<'_>,
    levels: &[u8],
    out: &mut Vec<ShapedSegment>,
) {
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
pub(super) struct PushRunArgs<'a> {
    pub(super) fragment: &'a str,
    pub(super) start_byte: usize,
    pub(super) level: u8,
    pub(super) script: Script,
    pub(super) is_break_opportunity: bool,
}

pub(super) fn push_run(
    args: &PushRunArgs<'_>,
    chain: &[Font],
    style: &Style,
    out: &mut Vec<ShapedSegment>,
) {
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

pub(super) fn shape_validated(
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

pub(super) fn run_has_disallowed_notdef(run: &GlyphRun, fragment: &str) -> bool {
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
        !grapheme
            .chars()
            .any(crate::text::shaper::is_default_ignorable)
    })
}

fn byte_level(levels: &[u8], byte: usize) -> u8 {
    levels.get(byte).copied().unwrap_or(0)
}

pub(super) fn char_script(ch: char) -> Script {
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
pub(super) const fn script_name(script: Script) -> &'static str {
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
pub(super) const fn u32_clamp(v: usize) -> u32 {
    if v > u32::MAX as usize {
        u32::MAX
    } else {
        v as u32
    }
}
