//! Owned text engine implementation (DR-044, G1.4).
//!
//! Pipeline: segment graphemes/words → bidi level resolution → skrifa
//! shaping per segment → word-boundary line breaking → line stacking.
//!
//! [`OwnedTextEngine`] is the public type. The G1.4–G1.6 source-compat
//! alias for the prior backend type was retired in G1.7 once every
//! in-tree caller had been migrated.

use std::cell::{Cell, Ref, RefCell};

use sigil_hal::{ParagraphMetrics, TextEngine};
use sigil_style::{StyledTextEngine, TextLineHeight, TextStyle};
use unicode_segmentation::UnicodeSegmentation;

use crate::bidi;
use crate::cache::ParagraphCache;
use crate::config::TextConfig;
use crate::error::TextError;
use crate::font_family::FontFamily;
use crate::fonts::DefaultFontSource;
use crate::glyph::GlyphRun;
use crate::paragraph::{LayoutLine, Paragraph, SegmentAppend, TextMetrics};
use crate::shaper::{Font, Script, Shaper, Style, TextSegment};

/// Owned text engine — skrifa + unicode-segmentation backed.
///
/// Holds the active font collection, a per-frame paragraph cache, and a
/// resolved default [`TextConfig`]. All interior mutability is via
/// [`RefCell`]; single-threaded by design (matches the existing engine
/// surface). `Send + Sync` is **not** required of the engine itself —
/// the underlying `Shaper` is, so future rayon use sites can be added
/// without revisiting this type.
pub struct OwnedTextEngine {
    fonts: RefCell<FontCollection>,
    /// Generation of the process-global default fonts this engine last shaped with.
    /// When it falls behind, [`Self::refresh_fonts`] rebuilds `fonts`.
    font_gen: Cell<u64>,
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
            fonts: RefCell::new(FontCollection::with_default()),
            font_gen: Cell::new(crate::fonts::default_fonts_generation()),
            config,
            cache: RefCell::new(ParagraphCache::new()),
        }
    }

    /// Rebuild the font collection when the process-global default fonts changed
    /// since this engine last shaped, and clear the paragraph cache so stale-face
    /// entries don't survive the swap. A cheap atomic load on the common path.
    fn refresh_fonts(&self) {
        let generation = crate::fonts::default_fonts_generation();
        if generation != self.font_gen.get() {
            *self.fonts.borrow_mut() = FontCollection::with_default();
            self.font_gen.set(generation);
            self.cache.borrow_mut().clear();
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
        // Pick up a runtime font swap (e.g. the debug panel A/B) before shaping.
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

    /// Lay out `text` as a single line truncated to fit `max_width`, with a
    /// trailing ellipsis (`…`) when it does not fit (CSS `text-overflow:
    /// ellipsis`). If the text already fits, the untruncated single-line
    /// paragraph is returned unchanged; if not even the ellipsis fits, the
    /// bare ellipsis is returned (the caller's clip handles the rest).
    ///
    /// The fitting prefix is found by binary search over character
    /// boundaries — `O(log n)` shaping passes, each served by the paragraph
    /// cache — so a long string costs a handful of measures, not one per
    /// character.
    ///
    /// # Errors
    ///
    /// Returns [`TextError`] if paragraph layout fails.
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

        // Character-boundary byte offsets, plus the end, as search points.
        let bounds: Vec<usize> = text
            .char_indices()
            .map(|(i, _)| i)
            .chain(std::iter::once(text.len()))
            .collect();
        // Largest `k` such that the prefix ending at `bounds[k]` fits `budget`.
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
        // Trim trailing spaces before the ellipsis so it reads as "word…".
        let kept = text.get(..bound_at(lo)).unwrap_or("").trim_end();
        let mut out = String::with_capacity(kept.len() + ELLIPSIS.len());
        out.push_str(kept);
        out.push_str(ELLIPSIS);
        self.layout_paragraph_styled(&out, None, style)
    }

    /// Borrow the font collection (for tests + font-registration).
    #[must_use]
    pub fn fonts(&self) -> Ref<'_, FontCollection> {
        self.fonts.borrow()
    }

    /// Debug-only segment snapshot for the integration test suite.
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
        self.fonts.borrow_mut().register(bytes, face_index)?;
        self.cache.borrow_mut().clear();
        Ok(())
    }

    /// Register a user-supplied font face for a named family, replacing
    /// the bundled default for that family (see
    /// [`FontCollection::register_family`] for recognised names).
    ///
    /// Unlike [`Self::register_font`] (which always replaces the sans
    /// primary), this targets one family slot so a caller can add e.g.
    /// a Hebrew face as `serif` for the per-run fallback chain
    /// (`FontCollection::fallback_chain`) to discover when the
    /// active style's family lacks coverage for a run.
    ///
    /// # Errors
    ///
    /// Returns [`TextError::InvalidFont`] when the bytes are
    /// unparseable or the family name is unknown.
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
        // Analysed once and reused for run segmentation (paragraph-wide
        // levels) and visual line reordering (line-aware L1 + L2) — see
        // `bidi::ParagraphBidi`.
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
            // font_size is the em-scale for ink; line_height is only row leading.
            // Use the shaper's size_px so ink matches the advances it produced.
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

/// Owned font collection.
///
/// Bootstrapped with the bundled `FiraSans` Regular face as the sans
/// primary and `CascadiaMono` Regular as the monospace face. Callers
/// can register additional families via [`Self::register_family`].
pub struct FontCollection {
    sans: FontFamily,
    mono: FontFamily,
    serif: Option<FontFamily>,
    icon: Option<Font>,
    /// Bundled Noto Color Emoji subset — the last-resort fallback face so
    /// emoji codepoints no text face covers resolve to a color glyph
    /// instead of tofu. Rendered through [`crate::color_glyph`], not Slug.
    emoji: Font,
}

impl FontCollection {
    /// Construct with the bootstrap sans + mono faces.
    ///
    /// Honors any process-global override registered via
    /// [`set_default_fonts`](crate::set_default_fonts) before falling back to
    /// the embedded `FiraSans` + `CascadiaMono` faces — including when an
    /// override's bytes fail to parse, so a malformed consumer font can never
    /// blank the UI.
    #[must_use]
    pub fn with_default() -> Self {
        let source = crate::fonts::default_font_source();
        if matches!(source, DefaultFontSource::System) {
            let system = crate::system_fonts::load();
            return Self {
                sans: system
                    .sans
                    .unwrap_or_else(|| FontFamily::new(load_sans_font())),
                mono: system
                    .mono
                    .unwrap_or_else(|| FontFamily::new(load_mono_font())),
                serif: system.serif,
                icon: None,
                emoji: load_emoji_font(),
            };
        }
        let over = match source {
            DefaultFontSource::Custom(fonts) => Some(fonts),
            DefaultFontSource::Bundled | DefaultFontSource::System => None,
        };
        let face = |slot: Option<&(std::sync::Arc<Vec<u8>>, u32)>| -> Option<Font> {
            let (bytes, index) = slot?;
            Font::from_shared(std::sync::Arc::clone(bytes), *index).ok()
        };
        Self {
            sans: FontFamily::new(
                over.as_ref()
                    .and_then(|f| face(f.sans.as_ref()))
                    .unwrap_or_else(load_sans_font),
            ),
            mono: FontFamily::new(
                over.as_ref()
                    .and_then(|f| face(f.mono.as_ref()))
                    .unwrap_or_else(load_mono_font),
            ),
            serif: None,
            // No bundled icon face — `select("icon")` falls back to sans until a
            // consumer registers one (e.g. the host app's codicon) via the
            // `DefaultFonts.icon` override.
            icon: over.as_ref().and_then(|f| face(f.icon.as_ref())),
            emoji: load_emoji_font(),
        }
    }

    /// Borrow the sans/primary face. Retained for compatibility with
    /// callers that don't yet pick a family — equivalent to `select("sans")`.
    #[must_use]
    pub fn primary(&self) -> &Font {
        self.sans.select(400)
    }

    /// Resolve a family name to a font handle. Unknown families fall
    /// back to the sans primary so unstyled callers always render
    /// something legible. Case-insensitive on the well-known names.
    #[must_use]
    pub fn select(&self, family: &str) -> &Font {
        self.select_weight(family, 400)
    }

    /// Resolve a family and CSS weight to the matching static or variable face.
    #[must_use]
    pub fn select_weight(&self, family: &str, weight: u16) -> &Font {
        let name = family.trim();
        if name.eq_ignore_ascii_case("mono") || name.eq_ignore_ascii_case("monospace") {
            self.mono.select(weight)
        } else if name.eq_ignore_ascii_case("serif") {
            self.serif.as_ref().unwrap_or(&self.sans).select(weight)
        } else if name.eq_ignore_ascii_case("icon") {
            self.icon
                .as_ref()
                .unwrap_or_else(|| self.sans.select(weight))
        } else {
            self.sans.select(weight)
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
        self.sans.replace(font);
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
            self.sans.replace(font);
        } else if name.eq_ignore_ascii_case("mono") || name.eq_ignore_ascii_case("monospace") {
            self.mono.replace(font);
        } else if name.eq_ignore_ascii_case("serif") {
            self.serif = Some(FontFamily::new(font));
        } else if name.eq_ignore_ascii_case("icon") {
            self.icon = Some(font);
        } else {
            return Err(TextError::InvalidFont);
        }
        Ok(())
    }

    /// Deterministic per-run fallback order for `primary`: `primary`
    /// itself first, then the collection's other distinct registered
    /// faces (sans, mono, serif, icon, in that fixed order) so a
    /// cluster missing from `primary` can still find coverage instead
    /// of rendering `.notdef` tofu (Wave 4 fallback chain). Faces are
    /// deduped by identity — registering the same bytes under two
    /// family names doesn't repeat them in the chain. `Font::clone` is
    /// an `Arc` bump, so the chain is cheap to build once per paragraph.
    #[must_use]
    pub(crate) fn fallback_chain(&self, primary: &Font, weight: u16) -> Vec<Font> {
        let mut chain: Vec<Font> = vec![primary.clone()];
        let candidates = [
            Some(self.sans.select(weight)),
            Some(self.mono.select(weight)),
            self.serif.as_ref().map(|family| family.select(weight)),
            self.icon.as_ref(),
            // Emoji last: only reached for codepoints no text face covers.
            Some(&self.emoji),
        ];
        for candidate in candidates.into_iter().flatten() {
            if chain.iter().all(|f| f.id() != candidate.id()) {
                chain.push(candidate.clone());
            }
        }
        chain
    }
}

/// Embedded `FiraSans` Regular — sans bootstrap face.
fn load_sans_font() -> Font {
    const BYTES: &[u8] = include_bytes!("../assets/fonts/FiraSans-Regular.ttf");
    Font::from_bytes(BYTES.to_vec(), 0)
        .expect("bundled FiraSans-Regular.ttf must parse — fixture validated at G1.3")
}

fn load_emoji_font() -> Font {
    const BYTES: &[u8] = include_bytes!("../assets/fonts/NotoColorEmoji-subset.ttf");
    Font::from_bytes(BYTES.to_vec(), 0)
        .expect("bundled NotoColorEmoji-subset.ttf must parse — validated in color_glyph tests")
}

/// Embedded `CascadiaMono` Regular — monospace bootstrap face. OFL
/// 1.1 (Microsoft, <https://github.com/microsoft/cascadia-code>).
fn load_mono_font() -> Font {
    const BYTES: &[u8] = include_bytes!("../assets/fonts/CascadiaMono-Regular.ttf");
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

/// Baseline geometry shared by every line in a paragraph — the strut
/// (ascent + descent, both positive px) the CSS half-leading model
/// centers each line box against. Bundled so the line-assembly
/// functions below stay under the 5-argument hygiene cap.
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
    /// `true` if a UAX #14 line break is permitted immediately BEFORE
    /// this segment's start byte. Computed once per paragraph by
    /// [`mark_break_opportunities`]; the greedy breaker may only wrap at
    /// segments where this holds, so glue (NBSP) and prohibited breaks
    /// (before closing punctuation) keep their neighbours together.
    breakable_before: bool,
    /// `true` if this segment is a hard line terminator (`\n`, `\r\n`,
    /// `\r`, U+2028, U+2029). Hard breaks are never shaped and always
    /// force a new line — set at segmentation time so the line breaker
    /// never has to guess from glyph metrics.
    is_hard_break: bool,
    /// UAX #9 bidi embedding level — even = LTR, odd = RTL. Travels
    /// through to the shaper so `HarfRust` sees the right direction,
    /// and drives the L2 visual reorder at line assembly
    /// ([`bidi::visual_order`]) since Wave 4.
    level: u8,
    /// Dominant script of this run (drives complex-shaper dispatch).
    /// Read only by the engine tests in G1.7 — kept on the segment
    /// because the next phase's logging path consumes it.
    #[allow(dead_code)]
    script: Script,
    /// Index into the paragraph's per-run font fallback chain — which
    /// face this segment was shaped with (Wave 4 per-glyph fallback).
    face_slot: u16,
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
///
/// `chain` is the paragraph's per-run font fallback chain
/// (`FontCollection::fallback_chain`) — `chain[0]` is the primary
/// face selected for the active style; each run independently picks
/// the first face in `chain` with full cmap coverage (Wave 4).
/// `paragraph_bidi` is the paragraph's precomputed bidi analysis
/// (shared with [`break_lines`] so `unicode_bidi`'s class-detection
/// pass runs once per paragraph, not once per line).
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
            // Hard line break: never shape, never measure, never
            // render. The line breaker will start a fresh line when
            // it sees one of these segments.
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

/// Fill each segment's `breakable_before` from UAX #14 line-break
/// analysis of the whole paragraph.
///
/// `unicode_linebreak::linebreaks` yields the byte index immediately
/// AFTER each opportunity, i.e. the index a break may fall BEFORE. We
/// record only `Allowed` (soft) opportunities in a byte-indexed table;
/// `Mandatory` breaks already travel as `is_hard_break` segments, so
/// folding them in here would double-count. A segment is breakable-
/// before when its start byte is such an allowed index — which is why
/// glue (NBSP), infix punctuation, and closing marks keep their
/// neighbours together even though they sit on word boundaries.
///
/// Face-fallback sub-splits land mid-run, never on a UAX #14 boundary,
/// so they correctly read `false` and never introduce a wrap inside a
/// word.
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

/// Recognise the UAX #14 mandatory line-break fragments that UAX #29
/// surfaces as their own word-boundary slices: `\n`, `\r\n`, `\r`,
/// U+2028 LINE SEPARATOR, U+2029 PARAGRAPH SEPARATOR. Form feed and
/// next-line are deliberately omitted — they are rare in editor text
/// and would expand the test matrix for no observed corpus.
fn is_line_terminator(fragment: &str) -> bool {
    matches!(fragment, "\n" | "\r\n" | "\r" | "\u{2028}" | "\u{2029}")
}

/// Shaping context held by the segmenter while it walks a word.
struct WordCtx<'a> {
    word: &'a str,
    word_start: usize,
    is_ws: bool,
    chain: &'a [Font],
    style: &'a Style,
}

/// Sub-segment `ctx.word` into maximal runs uniform in (bidi-level,
/// script). Each run is shaped and pushed onto `out` with the correct
/// level + script tag.
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

/// Shape `args.fragment`, splitting it further at grapheme-cluster
/// boundaries by which chain face covers each cluster. A single
/// uniform bidi/script run can still mix coverage across faces — e.g.
/// a base-text cluster the primary covers next to a symbol/emoji
/// cluster only a fallback face covers, both absorbed into one run
/// because their script classifies as `Common`/`Other` — so requiring
/// one face to cover the WHOLE run swallows it entirely to `.notdef`
/// even when the fallback chain collectively covers every cluster.
///
/// Adjacent clusters that pick the same face are coalesced into one
/// sub-run before shaping (grapheme-granularity, never finer — a
/// cluster's codepoints must always stay on the same face, since
/// splitting *inside* a grapheme cluster across two fonts is
/// shaping-nonsensical).
fn push_run(args: &PushRunArgs<'_>, chain: &[Font], style: &Style, out: &mut Vec<ShapedSegment>) {
    if args.fragment.is_empty() {
        return;
    }
    for span in coalesce_face_spans(chain, args.fragment) {
        push_face_span(args, chain, style, &span, out);
    }
}

/// One coalesced sub-run of `push_run`'s fragment: a byte range
/// (relative to the fragment) every grapheme cluster in which picked
/// the same fallback-chain face.
struct FaceSpan {
    start: usize,
    end: usize,
    face_slot: u16,
}

/// Segment `fragment` into [`FaceSpan`]s by per-grapheme-cluster face
/// selection ([`select_face`]), coalescing adjacent clusters that pick
/// the same face.
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

/// Shape one [`FaceSpan`] of `args.fragment` and push the resulting
/// [`ShapedSegment`]. Validates the shape ([`shape_validated`]) rather
/// than trusting the cmap-coverage pre-filter blindly (finding 2).
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
    let (face_slot, run) = shape_validated(chain, span.face_slot, sub_fragment, args.level, style);
    let advance = run_total_advance(&run);
    out.push(ShapedSegment {
        start: u32_clamp(start_byte),
        end: u32_clamp(end_byte),
        run,
        advance,
        is_break_opportunity: args.is_break_opportunity,
        // Set in the paragraph-wide post-pass, mark_break_opportunities.
        breakable_before: false,
        is_hard_break: false,
        level: args.level,
        script: args.script,
        face_slot,
    });
}

/// Pick the first face in `chain` with full cmap coverage of
/// `fragment` (see [`Font::covers`] for the default-ignorable
/// transparency), falling back to `chain[0]` (the paragraph's primary
/// face — rendering `.notdef` tofu) when no registered face covers
/// it. `chain` is never empty: `FontCollection::fallback_chain`
/// always seeds it with the primary face.
///
/// This is a cheap cmap pre-filter, not a shaping-success guarantee —
/// [`shape_validated`] is what actually confirms the pick worked.
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

/// Shape `fragment` with `chain[initial_slot]` and validate the result
/// has no disallowed `.notdef` glyph (a glyph clustered to a
/// non-default-ignorable byte); if it does, retry against the rest of
/// `chain` in order and take the first clean shape. Falls back to the
/// initial attempt — still better than nothing — if none is clean.
///
/// Closes the gap `select_face`'s cmap pre-filter can't: cmap presence
/// doesn't prove the shaped buffer avoids `.notdef` (a face can map a
/// codepoint but still fail to produce the glyph a script's GSUB rules
/// substitute).
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

/// Whether `run` contains a `.notdef` glyph whose grapheme cluster has
/// no default-ignorable codepoint (ZWJ/ZWNJ/VS15/VS16/CGJ/etc — see
/// [`crate::shaper::is_default_ignorable`]) that could explain it. A
/// `.notdef` on a default-ignorable control is expected — those carry
/// no visible glyph by design — so it doesn't disqualify the shape.
///
/// Checks the WHOLE grapheme cluster starting at `g.cluster`, not just
/// the codepoint at that exact byte offset: the owned (non-`HarfRust`)
/// shaper positions every codepoint of a multi-codepoint cluster (a
/// base plus a combining mark or ignorable control) at the SAME
/// `cluster` value — e.g. `"a" + VS17` clusters both glyphs at byte 0,
/// so checking only `fragment[0..]`'s first char would classify a
/// VS17-caused `.notdef` against `'a'` (non-ignorable) and wrongly
/// flag it.
///
/// ponytail: per-cluster (not per-glyph) attribution — a `.notdef` for
/// a genuinely-uncovered BASE character sharing a cluster with an
/// ignorable mark would also be (wrongly) excused here, since OpenType
/// clustering doesn't expose which glyph came from which codepoint.
/// Upgrade path: thread per-glyph source-codepoint provenance through
/// `GlyphPos` if a real fixture exercises this combination.
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

/// Segments collected for the line currently being assembled, in
/// logical (source) order. Wrap decisions only need the
/// order-independent sum of advances, so the visual reorder
/// ([`push_visual_line`]) happens once per completed line rather than
/// per segment.
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

/// Break `segments` into [`LayoutLine`]s honouring `max_width`.
///
/// Algorithm: greedy word-boundary fit. A non-whitespace segment that
/// would overflow the current line forces a wrap (the trailing
/// whitespace from the previous line is elided). When `max_width` is
/// `None` everything stays on one line modulo explicit `\n`.
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
    // Logical byte position for the line CURRENTLY being assembled,
    // used only when it turns out empty (consecutive hard breaks, or a
    // leading/trailing hard break) — a non-empty line's true byte
    // range comes from its own segments instead (see `push_visual_line`).
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
                // Elide the leading whitespace at the start of a wrapped line.
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
    // UAX #14: a soft wrap may only fall where a break is permitted
    // before this segment. Elidable whitespace never starts a line, and
    // glue / prohibited-break boundaries keep the run intact (overflowing
    // rather than splitting), matching CSS `overflow-wrap: normal`.
    if seg.is_break_opportunity || !seg.breakable_before {
        return false;
    }
    if pending.is_empty() {
        return false;
    }
    pending.width + seg.advance > limit
}

/// Assemble one [`LayoutLine`] from `pending`'s logical-order segments,
/// reordering them into UAX #9 visual (paint) order first.
///
/// Uses [`bidi::ParagraphBidi::visual_order_for_line`] — rule L1 (reset
/// trailing whitespace/separators to the paragraph level) for THIS
/// line's actual byte range, then rule L2 — rather than feeding
/// paragraph-resolved segment levels straight into L2: L1 depends on
/// where a line ends after word-wrapping, which a paragraph-wide
/// levels array can't know. Each segment is already uniform in bidi
/// level by construction, so reordering at segment granularity is
/// equivalent to reordering at codepoint granularity.
///
/// `empty_line_byte` is the true logical byte position to stamp on
/// `pending` when it turns out to be empty (consecutive hard breaks,
/// or a leading/trailing one) — see
/// [`LayoutLine::set_empty_byte_position`].
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
