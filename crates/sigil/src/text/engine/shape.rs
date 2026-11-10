use smallvec::SmallVec;
use unicode_segmentation::UnicodeSegmentation;

use crate::text::glyph::ShapedRun;
use crate::text::shaper::{FaceStyle, Font, Script};

mod cache;
mod face;

use cache::{WordSpan, remember, replay, word_key};
use face::{FaceSpan, cell_width, coalesce_face_spans, fit_cells, shape_in_chain};

mod script;

pub(super) use script::char_script;
#[cfg(any(test, feature = "kittest"))]
pub(super) use script::script_name;

pub(super) struct ShapedSegment {
    pub(super) start: u32,
    pub(super) end: u32,
    pub(super) run: ShapedRun,
    pub(super) advance: f32,
    pub(super) is_break_opportunity: bool,
    pub(super) breakable_before: bool,
    pub(super) is_hard_break: bool,
    pub(super) level: u8,
    #[cfg(any(test, feature = "kittest"))]
    pub(super) script: Script,
    pub(super) face_slot: u16,
}

pub(super) fn mark_break_opportunities(text: &str, segments: &mut [ShapedSegment]) {
    let mut allowed = SmallVec::<[bool; 128]>::from_elem(false, text.len().saturating_add(1));
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
    pub(super) style: &'a FaceStyle,
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
            #[cfg(any(test, feature = "kittest"))]
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
    #[cfg(any(test, feature = "kittest"))]
    pub(super) script: Script,
    pub(super) is_break_opportunity: bool,
}

const WORD_MAX_BYTES: usize = 64;

pub(super) fn push_run(
    args: &PushRunArgs<'_>,
    chain: &[Font],
    style: &FaceStyle,
    out: &mut Vec<ShapedSegment>,
) {
    if args.fragment.is_empty() {
        return;
    }
    if args.fragment.len() > WORD_MAX_BYTES {
        shape_face_spans(args, chain, style, out);
        return;
    }
    let key = word_key(args, chain, style);
    if replay(key, args, chain, style, out) {
        return;
    }
    let first = out.len();
    shape_face_spans(args, chain, style, out);
    remember(
        key,
        args,
        chain,
        style,
        out.get(first..).unwrap_or_default(),
    );
}

fn shape_face_spans(
    args: &PushRunArgs<'_>,
    chain: &[Font],
    style: &FaceStyle,
    out: &mut Vec<ShapedSegment>,
) {
    for span in coalesce_face_spans(chain, args.fragment) {
        push_face_span(args, chain, style, &span, out);
    }
}

fn push_face_span(
    args: &PushRunArgs<'_>,
    chain: &[Font],
    style: &FaceStyle,
    span: &FaceSpan,
    out: &mut Vec<ShapedSegment>,
) {
    let Some(sub_fragment) = args.fragment.get(span.start..span.end) else {
        return;
    };
    if sub_fragment.is_empty() {
        return;
    }
    let (face_slot, mut run) =
        shape_validated(chain, span.face_slot, sub_fragment, args.level, style);
    apply_letter_spacing(&mut run, style);
    let advance = run_total_advance(&run);
    let span = WordSpan {
        start: u32_clamp(span.start),
        end: u32_clamp(span.end),
        face_slot,
        run,
        advance,
    };
    out.push(segment(args, span));
}

fn segment(args: &PushRunArgs<'_>, span: WordSpan) -> ShapedSegment {
    let base = u32_clamp(args.start_byte);
    ShapedSegment {
        start: base.saturating_add(span.start),
        end: base.saturating_add(span.end),
        run: span.run,
        advance: span.advance,
        is_break_opportunity: args.is_break_opportunity,
        breakable_before: false,
        is_hard_break: false,
        level: args.level,
        #[cfg(any(test, feature = "kittest"))]
        script: args.script,
        face_slot: span.face_slot,
    }
}

pub(super) fn shape_validated(
    chain: &[Font],
    initial_slot: u16,
    fragment: &str,
    level: u8,
    style: &FaceStyle,
) -> (u16, ShapedRun) {
    let (slot, mut run) = shape_in_chain(chain, initial_slot, fragment, level, style);
    if style.cell_grid && slot != 0 {
        fit_cells(&mut run, cell_width(chain, style));
    }
    (slot, run)
}

pub(super) fn run_has_disallowed_notdef(run: &ShapedRun, fragment: &str) -> bool {
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

fn apply_letter_spacing(run: &mut ShapedRun, style: &FaceStyle) {
    let extra = style.letter_spacing_em * style.size_px;
    if extra.abs() < f32::EPSILON {
        return;
    }
    for glyph in &mut run.glyphs {
        glyph.advance += extra;
    }
}

fn run_total_advance(run: &ShapedRun) -> f32 {
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
