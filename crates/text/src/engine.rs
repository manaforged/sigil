//! Owned text engine implementation (DR-044, G1.4).
//!
//! Pipeline: segment graphemes/words → bidi level resolution → skrifa
//! shaping per segment → word-boundary line breaking → line stacking.
//!
//! [`OwnedTextEngine`] is the public type. The G1.4–G1.6 source-compat
//! alias for the prior backend type was retired in G1.7 once every
//! in-tree caller had been migrated.

use std::cell::RefCell;

use hal::TextEngine;
use style::{StyledTextEngine, TextLineHeight, TextStyle};
use unicode_segmentation::UnicodeSegmentation;

use crate::bidi;
use crate::cache::ParagraphCache;
use crate::config::TextConfig;
use crate::error::TextError;
use crate::glyph::GlyphRun;
use crate::paragraph::{LayoutLine, Paragraph};
use crate::shaper::{Font, Script, Shaper, Style, TextSegment};

/// Owned text engine — skrifa + unicode-segmentation backed.
///
/// Holds the active font collection, a per-frame paragraph cache, and a
/// resolved default [`TextConfig`]. All interior mutability is via
/// [`RefCell`]; single-threaded by design (matches the existing engine
/// surface). `Send + Sync` is **not** required of the engine itself —
/// the underlying [`Shaper`] is, so future rayon use sites can be added
/// without revisiting this type.
pub struct OwnedTextEngine {
    fonts: FontCollection,
    config: TextConfig,
    cache: RefCell<ParagraphCache>,
}

impl OwnedTextEngine {
    /// Construct an engine with `config` as the default style fallback.
    ///
    /// The font collection is bootstrapped with the bundled `FiraSans`
    /// Regular face — the same fixture the test suite uses. Production
    /// callers can replace it via [`FontCollection::register`] once the
    /// font-discovery surface lands (DR-044 open question 2).
    #[must_use]
    pub fn new(config: TextConfig) -> Self {
        Self {
            fonts: FontCollection::with_default(),
            config,
            cache: RefCell::new(ParagraphCache::new()),
        }
    }

    /// Clear the paragraph cache. Call once per frame before layout.
    pub fn clear_cache(&self) {
        self.cache.borrow_mut().clear();
    }

    /// Lay out a paragraph with explicit resolved typography.
    ///
    /// # Errors
    ///
    /// Returns [`TextError`] if paragraph layout fails. The current
    /// owned path is infallible — kept for forward compatibility with
    /// a future fallible font-discovery path.
    pub fn layout_paragraph_styled(
        &self,
        text: &str,
        max_width: Option<f32>,
        style: &TextStyle,
    ) -> Result<Paragraph, TextError> {
        if let Some(cached) = self.cache.borrow().get(text, max_width, style) {
            return Ok(cached.clone());
        }
        let para = self.build_paragraph(text, max_width, style);
        self.cache
            .borrow_mut()
            .insert(text, max_width, style, para.clone());
        Ok(para)
    }

    /// Borrow the font collection (for tests + font-registration).
    #[must_use]
    pub const fn fonts(&self) -> &FontCollection {
        &self.fonts
    }

    /// Debug-only segment snapshot for the integration test suite.
    #[doc(hidden)]
    #[cfg(any(test, debug_assertions))]
    #[must_use]
    pub fn debug_segments_for_tests(&self, text: &str) -> Vec<(u32, u32, u8, &'static str)> {
        let font = self.fonts.primary();
        let shaper = Shaper::new(font);
        let style = to_shaper_style(&self.config.text_style());
        shape_segments(&shaper, text, &style)
            .into_iter()
            .map(|s| (s.start, s.end, s.level, script_name(s.script)))
            .collect()
    }

    /// Register a user-supplied font face, replacing the bundled
    /// fallback.
    ///
    /// `bytes` must be a valid TTF/OTF face; `face_index` selects the
    /// face inside a TTC (pass `0` for single-face files). On success
    /// subsequent calls to [`Self::layout_paragraph`] and
    /// [`Self::layout_paragraph_styled`] resolve glyphs against the
    /// newly-registered face.
    ///
    /// Registering also clears the paragraph cache — entries from the
    /// previous face would otherwise survive into the next frame.
    ///
    /// # Errors
    ///
    /// Returns [`TextError::InvalidFont`] when the bytes do not parse
    /// as a valid OpenType/TrueType face.
    pub fn register_font(&mut self, bytes: Vec<u8>, face_index: u32) -> Result<(), TextError> {
        self.fonts.register(bytes, face_index)?;
        self.cache.borrow_mut().clear();
        Ok(())
    }

    fn build_paragraph(&self, text: &str, max_width: Option<f32>, style: &TextStyle) -> Paragraph {
        let font = self.fonts.select(&style.font_family);
        let shaper = Shaper::new(font);
        let shaper_style = to_shaper_style(style);
        let line_height = resolve_line_height(style);
        let runs = shape_segments(&shaper, text, &shaper_style);
        let min_content_hint = runs
            .iter()
            .filter(|s| !s.is_break_opportunity)
            .map(|s| s.advance)
            .fold(0.0_f32, f32::max);
        let lines = break_lines(&runs, max_width, line_height);
        Paragraph::new(
            lines,
            line_height,
            font.bytes_arc(),
            font.face_index(),
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

/// Owned font collection.
///
/// Bootstrapped with the bundled `FiraSans` Regular face as the sans
/// primary and `CascadiaMono` Regular as the monospace face. Callers
/// can register additional families via [`Self::register_family`].
pub struct FontCollection {
    sans: Font,
    mono: Font,
    serif: Option<Font>,
}

impl FontCollection {
    /// Construct with the embedded sans + mono faces.
    #[must_use]
    pub fn with_default() -> Self {
        Self {
            sans: load_sans_font(),
            mono: load_mono_font(),
            serif: None,
        }
    }

    /// Borrow the sans/primary face. Retained for compatibility with
    /// callers that don't yet pick a family — equivalent to `select("sans")`.
    #[must_use]
    pub const fn primary(&self) -> &Font {
        &self.sans
    }

    /// Resolve a family name to a font handle. Unknown families fall
    /// back to the sans primary so unstyled callers always render
    /// something legible. Case-insensitive on the well-known names.
    #[must_use]
    pub fn select(&self, family: &str) -> &Font {
        let name = family.trim();
        if name.eq_ignore_ascii_case("mono") || name.eq_ignore_ascii_case("monospace") {
            &self.mono
        } else if name.eq_ignore_ascii_case("serif") {
            self.serif.as_ref().unwrap_or(&self.sans)
        } else {
            &self.sans
        }
    }

    /// Replace the sans/primary face with a user-supplied font.
    ///
    /// # Errors
    ///
    /// Returns [`TextError::InvalidFont`] when the bytes do not parse as
    /// a valid OpenType/TrueType face.
    pub fn register(&mut self, bytes: Vec<u8>, face_index: u32) -> Result<(), TextError> {
        let font = Font::from_bytes(bytes, face_index).map_err(|_| TextError::InvalidFont)?;
        self.sans = font;
        Ok(())
    }

    /// Replace the face for a named family. Recognised names:
    /// `sans` / `sans-serif` / `system-ui`, `mono` / `monospace`,
    /// `serif`. Unknown names return [`TextError::InvalidFont`].
    ///
    /// # Errors
    ///
    /// Returns [`TextError::InvalidFont`] when the bytes are unparseable
    /// or the family name is unknown.
    pub fn register_family(
        &mut self,
        family: &str,
        bytes: Vec<u8>,
        face_index: u32,
    ) -> Result<(), TextError> {
        let font = Font::from_bytes(bytes, face_index).map_err(|_| TextError::InvalidFont)?;
        let name = family.trim();
        if name.eq_ignore_ascii_case("sans")
            || name.eq_ignore_ascii_case("sans-serif")
            || name.eq_ignore_ascii_case("system-ui")
        {
            self.sans = font;
        } else if name.eq_ignore_ascii_case("mono") || name.eq_ignore_ascii_case("monospace") {
            self.mono = font;
        } else if name.eq_ignore_ascii_case("serif") {
            self.serif = Some(font);
        } else {
            return Err(TextError::InvalidFont);
        }
        Ok(())
    }
}

/// Embedded `FiraSans` Regular — sans bootstrap face.
fn load_sans_font() -> Font {
    const BYTES: &[u8] = include_bytes!("../tests/fixtures/FiraSans-Regular.ttf");
    Font::from_bytes(BYTES.to_vec(), 0)
        .expect("bundled FiraSans-Regular.ttf must parse — fixture validated at G1.3")
}

/// Embedded `CascadiaMono` Regular — monospace bootstrap face. OFL
/// 1.1 (Microsoft, <https://github.com/microsoft/cascadia-code>).
fn load_mono_font() -> Font {
    const BYTES: &[u8] = include_bytes!("../tests/fixtures/CascadiaMono-Regular.ttf");
    Font::from_bytes(BYTES.to_vec(), 0).expect("bundled CascadiaMono-Regular.ttf must parse")
}

/// Translate the resolved style into shaper-native parameters.
fn to_shaper_style(style: &TextStyle) -> Style {
    Style {
        size_px: style.font_size.max(1.0),
        weight: style.font_weight,
        italic: false,
        features: smallvec::SmallVec::new(),
    }
}

/// Resolve the effective line height in logical pixels.
fn resolve_line_height(style: &TextStyle) -> f32 {
    match style.line_height {
        TextLineHeight::FontSizeRelative(factor) => (style.font_size * factor.max(0.1)).max(1.0),
        TextLineHeight::Absolute(value) => value.max(1.0),
    }
}

/// One shaped segment between two break opportunities.
struct ShapedSegment {
    /// Byte range in the source string `[start, end)`.
    start: u32,
    end: u32,
    /// Glyph run produced by the shaper.
    run: GlyphRun,
    /// Total horizontal advance of the segment.
    advance: f32,
    /// `true` if the trailing whitespace can be elided at a line break.
    is_break_opportunity: bool,
    /// `true` if this segment is a hard line terminator (`\n`, `\r\n`,
    /// `\r`, U+2028, U+2029). Hard breaks are never shaped and always
    /// force a new line — set at segmentation time so the line breaker
    /// never has to guess from glyph metrics.
    is_hard_break: bool,
    /// UAX #9 bidi embedding level — even = LTR, odd = RTL.
    /// Travels through to the shaper so `HarfRust` sees the right
    /// direction. Visual reorder (G2) consumes this; in G1.7 it is
    /// read only by the engine tests via `segments_for_tests`.
    #[allow(dead_code)]
    level: u8,
    /// Dominant script of this run (drives complex-shaper dispatch).
    /// Read only by the engine tests in G1.7 — kept on the segment
    /// because the next phase's logging + reorder paths consume it.
    #[allow(dead_code)]
    script: Script,
}

/// Shape `text` honouring word boundaries, bidi runs, and script runs.
///
/// Each emitted [`ShapedSegment`] is uniform in:
/// - UAX #29 word boundary (no segment crosses a word-bound index)
/// - UAX #9 bidi embedding level (LTR vs RTL run)
/// - UAX #24 script (Latin / Arabic / Hebrew / CJK / …)
///
/// Whitespace-only segments mark explicit break opportunities so the
/// line breaker can elide them at wrap points. The bidi level travels
/// through to the shaper so the `HarfRust` dispatch + skrifa shaper
/// see the correct direction.
fn shape_segments(shaper: &Shaper<'_>, text: &str, style: &Style) -> Vec<ShapedSegment> {
    if text.is_empty() {
        return Vec::new();
    }

    let levels = bidi::paragraph_levels(text);

    let mut out: Vec<ShapedSegment> = Vec::new();
    for (word_start, fragment) in text.split_word_bound_indices() {
        if fragment.is_empty() {
            continue;
        }
        if is_line_terminator(fragment) {
            // Hard line break: never shape, never measure, never
            // render. The line breaker will start a fresh line when
            // it sees one of these segments.
            out.push(ShapedSegment {
                start: u32_clamp(word_start),
                end: u32_clamp(word_start + fragment.len()),
                run: GlyphRun::empty(),
                advance: 0.0,
                is_break_opportunity: true,
                is_hard_break: true,
                level: 0,
                script: Script::Common,
            });
            continue;
        }
        let ctx = WordCtx {
            word: fragment,
            word_start,
            is_ws: fragment.chars().all(char::is_whitespace),
            shaper,
            style,
        };
        split_word_into_uniform_runs(&ctx, &levels, &mut out);
    }
    out
}

/// Recognise the UAX #14 mandatory line-break fragments that UAX #29
/// surfaces as their own word-boundary slices: `\n`, `\r\n`, `\r`,
/// U+2028 LINE SEPARATOR, U+2029 PARAGRAPH SEPARATOR. Form feed and
/// next-line are deliberately omitted — they are rare in editor text
/// and would expand the test matrix for no observed corpus.
fn is_line_terminator(fragment: &str) -> bool {
    matches!(fragment, "\n" | "\r\n" | "\r" | "\u{2028}" | "\u{2029}")
}

/// Shaping context held by the segmenter while it walks a word.
struct WordCtx<'a, 'b> {
    word: &'a str,
    word_start: usize,
    is_ws: bool,
    shaper: &'a Shaper<'b>,
    style: &'a Style,
}

/// Sub-segment `ctx.word` into maximal runs uniform in (bidi-level,
/// script). Each run is shaped and pushed onto `out` with the correct
/// level + script tag.
fn split_word_into_uniform_runs(
    ctx: &WordCtx<'_, '_>,
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

/// Rolling state for the bidi/script segmenter inside one word.
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
        // Promote Common/Other to the first decisive script we see.
        if let Some(cur) = self.script
            && matches!(cur, Script::Common | Script::Other)
            && !matches!(script, Script::Common | Script::Other)
        {
            self.script = Some(script);
        }
    }

    fn flush_to(&self, ctx: &WordCtx<'_, '_>, end_offset: usize, out: &mut Vec<ShapedSegment>) {
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
        push_run(&args, ctx.shaper, ctx.style, out);
    }
}

/// Two adjacent codepoints belong to the same uniform run iff they
/// share a bidi level AND share a script — with `Common` / `Other`
/// scripts (spaces, punctuation, digits) absorbed into the surrounding
/// script run rather than splitting it.
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

fn push_run(
    args: &PushRunArgs<'_>,
    shaper: &Shaper<'_>,
    style: &Style,
    out: &mut Vec<ShapedSegment>,
) {
    if args.fragment.is_empty() {
        return;
    }
    let end_byte = args.start_byte + args.fragment.len();
    let seg = TextSegment::at_level(args.fragment, args.level);
    let run = shaper.shape(&seg, style);
    let advance = run_total_advance(&run);
    out.push(ShapedSegment {
        start: u32_clamp(args.start_byte),
        end: u32_clamp(end_byte),
        run,
        advance,
        is_break_opportunity: args.is_break_opportunity,
        is_hard_break: false,
        level: args.level,
        script: args.script,
    });
}

/// Lookup the bidi level for the byte at `byte`, returning `0` when
/// the index is past the end of `levels` (defensive — should never
/// fire because `levels.len() == text.len()`).
fn byte_level(levels: &[u8], byte: usize) -> u8 {
    levels.get(byte).copied().unwrap_or(0)
}

/// Classify a single char into our coarse `Script` enum. Mirrors the
/// per-segment classifier in `shaper::classify` but operates on a
/// single codepoint so the engine can split before shaping.
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

fn run_total_advance(run: &GlyphRun) -> f32 {
    run.glyphs.iter().map(|g| g.advance).sum()
}

#[allow(clippy::cast_possible_truncation)]
const fn u32_clamp(v: usize) -> u32 {
    if v > u32::MAX as usize {
        u32::MAX
    } else {
        v as u32
    }
}

/// Break `segments` into [`LayoutLine`]s honouring `max_width`.
///
/// Algorithm: greedy word-boundary fit. A non-whitespace segment that
/// would overflow the current line forces a wrap (the trailing
/// whitespace from the previous line is elided). When `max_width` is
/// `None` everything stays on one line modulo explicit `\n`.
fn break_lines(
    segments: &[ShapedSegment],
    max_width: Option<f32>,
    line_height: f32,
) -> Vec<LayoutLine> {
    if segments.is_empty() {
        return vec![LayoutLine::empty(0, line_height)];
    }
    let mut lines: Vec<LayoutLine> = Vec::new();
    let mut current = LayoutLine::empty(0, line_height);
    for seg in segments {
        if seg.is_hard_break {
            finalize_line(&mut lines, current, line_height);
            current =
                LayoutLine::empty(u32::try_from(lines.len()).unwrap_or(u32::MAX), line_height);
            continue;
        }
        if should_wrap(&current, seg, max_width) {
            finalize_line(&mut lines, current, line_height);
            current =
                LayoutLine::empty(u32::try_from(lines.len()).unwrap_or(u32::MAX), line_height);
            if seg.is_break_opportunity {
                // Elide the leading whitespace at the start of a wrapped line.
                continue;
            }
        }
        current.append_segment(&seg.run, seg.advance, seg.start, seg.end);
    }
    finalize_line(&mut lines, current, line_height);
    lines
}

fn should_wrap(current: &LayoutLine, seg: &ShapedSegment, max_width: Option<f32>) -> bool {
    let Some(limit) = max_width else {
        return false;
    };
    if seg.is_break_opportunity {
        return false;
    }
    if current.is_empty() {
        return false;
    }
    current.width() + seg.advance > limit
}

fn finalize_line(out: &mut Vec<LayoutLine>, mut line: LayoutLine, line_height: f32) {
    line.set_baseline_to(line_height);
    out.push(line);
}
