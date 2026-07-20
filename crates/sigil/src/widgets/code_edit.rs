//! `code_edit` — a multi-line, **no-wrap** code surface.
//!
//! Renders an arbitrary buffer as one monospace row per line (long lines clip rather
//! than wrap). Each row is painted as colored runs from the optional per-line
//! [`HlSpan`]s; the caret is a block drawn *between glyphs* by cutting the row at the
//! caret column, so the text engine positions it exactly. The widget owns **no buffer**
//! — it renders the `lines` / `caret` / `highlights` signals and forwards keys to
//! the `key` handler, so the consumer keeps one source of truth (e.g. an engine rope) elsewhere.
//!
//! Split by concept: this module is the public builder + shared row/window types;
//! [`scroll`] owns viewport virtualization; [`render`] paints each row.

mod render;
mod scroll;

use std::rc::Rc;

use sigil_events::prelude::{KeyEvent, NodeEventsExt};
use sigil_kernel::{Cx, IntoNode, NodeBuilder, NodeId, Role, container};
use sigil_layout::LayoutReactiveExt;
use sigil_layout::prelude::{LayoutBuilderExt, LayoutStyle, Overflow};
use sigil_reactive::{Signal, derived_eq, effect, signal};
use sigil_style::NodeStyleExt;

use scroll::{first_row, follow_caret, scroll_body, wheel};

/// Default row height in logical px — `text-sm` mono (14px × ~1.3 leading).
const ROW_H: f32 = 18.0;
/// Caret bar width in logical px.
const CARET_W: f32 = 2.0;
/// Width of the per-line change bar painted at the left edge of the gutter.
const CHANGE_BAR_W: f32 = 2.0;
/// Approx advance of one `text-sm` mono digit — sizes the line-number column.
const DIGIT_W: f32 = 7.0;
/// Padding flanking the line-number text inside the gutter.
const GUTTER_PAD: f32 = 8.0;
/// Background token for a selected glyph run. `accent` is the theme's subtle raised
/// surface — visible against the editor background without fighting the syntax colors.
const SELECTION_CLASS: &str = "bg-accent";
/// Size + font for every glyph run. Color is applied per-run via a theme token (so a
/// syntax span can recolor without fighting a `text-*` color class).
const RUN_CLASS: &str = "text-sm font-mono";
/// Extra rows materialized beyond the visible count, so a partial row at the bottom and a
/// little scroll headroom are always present. Added to the live visible-row count (derived
/// from the real viewport height via `size_reactive`) to form the per-frame render budget.
const ROW_BUFFER: usize = 6;
/// Viewport height assumed until the first layout publishes the real one (a fresh widget's
/// `size_reactive` signal starts at 0). Deliberately generous: OVER-filling the first
/// frame just builds a few extra rows the viewport clips (cheap, corrected next frame),
/// whereas under-filling shows a blank strip at the bottom of a tall pane for one frame.
const DEFAULT_VIEWPORT_H: f32 = 1600.0;

type KeyHandler = Rc<dyn Fn(&mut KeyEvent) + 'static>;

/// The selection anchor signal: `Some((line, col))` while a range is selected, `None` for a
/// bare caret. The head is the caret signal, so the range is `anchor..caret`.
type SelectionAnchor = Signal<Option<(usize, usize)>>;
/// An ordered selection range as `(start, end)` `(line, col)` positions, `start <= end`.
type SelRange = ((usize, usize), (usize, usize));

/// Per-line change marks for the gutter: `marks[i]` is the change bar for line `i`
/// (`None` = unchanged). A shorter/empty vec just means the trailing lines are unchanged,
/// so a consumer can show line numbers only by binding an empty vec.
type GutterMarks = Signal<Vec<Option<ChangeKind>>>;

/// How a line differs from its baseline (e.g. git HEAD), shown as a colored gutter bar.
///
/// Renderer-neutral: the consumer decides what "changed" means (git, agent edit, …); the
/// widget only paints the bar. `Deleted` marks the line *below* which one or more baseline
/// lines were removed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChangeKind {
    /// A new line with no baseline counterpart.
    Added,
    /// A line whose content changed from the baseline.
    Modified,
    /// Content was removed after this line (a deletion wedge anchor).
    Deleted,
}

impl ChangeKind {
    /// The `bg-*` utility class painting this change bar — green add, blue modify, red
    /// delete, on the theme's solid (step-9) semantic scales.
    const fn bar_class(self) -> &'static str {
        match self {
            Self::Added => "bg-success-9",
            Self::Modified => "bg-info-9",
            Self::Deleted => "bg-error-9",
        }
    }
}

/// A syntax-highlight span over one line: chars `[start, end)` painted with `token`.
///
/// Renderer-neutral — the consumer maps its own highlighter's capture names to theme
/// color tokens (e.g. `"syntax-keyword"`), so the widget needs no language knowledge.
#[derive(Clone, Debug)]
pub struct HlSpan {
    /// First char (inclusive) of the run within its line.
    pub start: usize,
    /// One-past-last char (exclusive) of the run within its line.
    pub end: usize,
    /// Theme color token to paint the run with (e.g. `"syntax-keyword"`).
    pub token: &'static str,
}

/// How serious a diagnostic is. Declared least-to-most severe so `Ord` picks the worst when
/// ranges overlap on a glyph (an error tint wins over a warning).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DiagSeverity {
    /// A hint (least severe) — e.g. an unnecessary cast.
    Hint,
    /// Informational — e.g. an unused binding the server only notes.
    Info,
    /// A warning — code that compiles but is suspect.
    Warning,
    /// An error (most severe) — code that does not compile / is invalid.
    Error,
}

impl DiagSeverity {
    /// The subtle `bg-*` tint painted behind a diagnosed run. A faint severity-colored band
    /// (step-3) reads as "something is wrong here" without a wavy-underline primitive (which
    /// the renderer does not yet have); it sits under the glyphs like the selection band.
    const fn tint_class(self) -> &'static str {
        match self {
            Self::Error => "bg-error-3",
            Self::Warning => "bg-warn-3",
            Self::Info | Self::Hint => "bg-info-3",
        }
    }
}

/// One diagnostic's underline range: chars `[start, end)` on `line`, at `severity`. Char-
/// indexed within the line, like [`HlSpan`]. The consumer maps LSP diagnostics to these.
#[derive(Clone, Copy, Debug)]
pub struct DiagSpan {
    /// Zero-based line the diagnostic is on.
    pub line: usize,
    /// First char (inclusive) of the diagnosed range within the line.
    pub start: usize,
    /// One-past-last char (exclusive) of the diagnosed range.
    pub end: usize,
    /// How serious the diagnostic is (drives the tint color).
    pub severity: DiagSeverity,
}

/// Per-document diagnostic spans, painted as severity-tinted bands under the glyphs.
type DiagSpans = Signal<Vec<DiagSpan>>;

/// Builder returned by [`code_edit`].
pub struct CodeEditBuilder {
    lines: Signal<Vec<String>>,
    caret: Signal<(usize, usize)>,
    highlights: Option<Signal<Vec<Vec<HlSpan>>>>,
    /// The selection ANCHOR (`(line, col)`) when a range is selected; `None` is a bare
    /// caret. The head is the `caret` signal, so a selection is `anchor..caret`. Painted as
    /// a background band behind the selected glyphs.
    selection: Option<SelectionAnchor>,
    /// When set, the gutter (right-aligned line numbers + per-line change bars) is drawn
    /// to the left of the text. `None` = no gutter (a bare read-only surface).
    gutter: Option<GutterMarks>,
    /// Diagnostic spans painted as severity-tinted bands under the glyphs; `None` = none.
    diagnostics: Option<DiagSpans>,
    on_key: Option<KeyHandler>,
    row_h: f32,
}

/// Create a no-wrap code surface over a `lines` signal and a `(line, col)` caret.
///
/// The caret column is a **char** offset within its line. Every signal is read
/// reactively: pushing new lines, moving the caret, or new highlights repaints.
#[must_use]
pub fn code_edit(lines: Signal<Vec<String>>, caret: Signal<(usize, usize)>) -> CodeEditBuilder {
    CodeEditBuilder {
        lines,
        caret,
        highlights: None,
        selection: None,
        gutter: None,
        diagnostics: None,
        on_key: None,
        row_h: ROW_H,
    }
}

impl CodeEditBuilder {
    /// Bind the selection anchor. When the signal is `Some((line, col))` and differs from
    /// the caret, the range `anchor..caret` is painted as a selection band; `None` is a bare
    /// caret. Char-indexed, like the caret.
    #[must_use]
    pub const fn selection(mut self, anchor: Signal<Option<(usize, usize)>>) -> Self {
        self.selection = Some(anchor);
        self
    }

    /// Run `f` for every key event that reaches the focused surface. The consumer
    /// maps keys to edits (insert `event.text`, Backspace/Delete, arrows, …) against
    /// its own buffer, then updates the `lines`/`caret` signals.
    #[must_use]
    pub fn key(mut self, f: impl Fn(&mut KeyEvent) + 'static) -> Self {
        self.on_key = Some(Rc::new(f));
        self
    }

    /// Per-line syntax spans painted under the text: `spans[i]` colors line `i`.
    /// Spans are assumed non-overlapping and char-indexed within their line.
    #[must_use]
    pub const fn highlights(mut self, spans: Signal<Vec<Vec<HlSpan>>>) -> Self {
        self.highlights = Some(spans);
        self
    }

    /// Override the per-row height (defaults to a `text-sm` mono line box).
    #[must_use]
    pub const fn row_height(mut self, h: f32) -> Self {
        self.row_h = h;
        self
    }

    /// Show the gutter: right-aligned line numbers plus a per-line change bar from
    /// `marks` (`marks[i]` paints line `i`; `None`/absent = unchanged). Bind an empty
    /// vec for line numbers only.
    #[must_use]
    pub const fn gutter(mut self, marks: GutterMarks) -> Self {
        self.gutter = Some(marks);
        self
    }

    /// Paint diagnostic bands: each [`DiagSpan`] tints its char range by severity. Char-
    /// indexed within the line; overlapping spans paint the most severe.
    #[must_use]
    pub const fn diagnostics(mut self, spans: DiagSpans) -> Self {
        self.diagnostics = Some(spans);
        self
    }

    fn into_node(self) -> NodeBuilder {
        let lines = self.lines;
        let caret = self.caret;
        let highlights = self.highlights;
        let selection = self.selection;
        let gutter = self.gutter;
        let diagnostics = self.diagnostics;
        let row_h = self.row_h;
        // Only trap Tab when an editing handler is installed. A read-only code surface
        // (no `on_key`) must NOT swallow Tab — that would strand focus traversal.
        let traps_tab = self.on_key.is_some();
        let on_key = self.on_key.unwrap_or_else(|| Rc::new(|_| {}));
        // Vertical scroll offset in logical px, owned by the widget. The wheel writes it;
        // a fresh widget per file (the consumer rebuilds on open) starts at the top.
        let scroll_y = signal(0.0_f32);
        // The viewport's own computed size, published by `size_reactive` after each layout
        // (height is what we use). Lets virtualization size its window to the REAL pane
        // height instead of a fixed guess — so a tall window never shows blank rows and a
        // short one never over-renders. Seeded to a sensible height for the first frame.
        let viewport = signal((0.0_f32, DEFAULT_VIEWPORT_H));
        // First visible row, snapped to a line boundary. `derived_eq` so the row window
        // rebuilds only when the integer row changes (a wheel tick inside one row is a
        // no-op), and so a clean frame between row steps stays clean. Reads `lines.get()`
        // (NOT peek) so swapping the buffer to a shorter file re-clamps `start` instead of
        // leaving it stranded past the new end (no cycle: `start` only feeds the rows).
        let start = derived_eq(move || first_row(scroll_y.get(), lines.get().len(), row_h));
        // Keep the caret in view: when it MOVES out of the visible band (typing, a jump),
        // snap the scroll so its row is shown. Event-driven on `caret`, so the user can
        // still freely wheel-scroll away from the caret without it yanking back.
        effect(move || follow_caret(scroll_y, caret, lines, viewport, row_h));
        // The viewport fills its pane (`w-full h-full`) and **clips**: a long line
        // overflows on the right and is scissored (never soft-wraps), and rows below the
        // window are scissored too. Clipping keeps the hit-box inside the pane.
        container()
            .role(Role::TextInput)
            .label("Code editor")
            .focusable(true)
            // Receive a plain Tab to insert an indent instead of letting the router
            // move focus (Shift+Tab still escapes, so this is no keyboard trap).
            .traps_tab(traps_tab)
            .class("w-full h-full")
            .layout(
                LayoutStyle::flex_column()
                    .overflow_x(Overflow::Hidden)
                    .overflow_y(Overflow::Hidden),
            )
            .size_reactive(viewport)
            .on_scroll(move |ev| wheel(ev, scroll_y, lines, viewport, row_h))
            .key_any(move |event| on_key(event))
            .child(scroll_body(
                Surface {
                    lines,
                    caret,
                    highlights,
                    selection,
                    gutter,
                    diagnostics,
                    start,
                    viewport,
                },
                row_h,
            ))
    }
}

/// The reactive inputs the row window reads, bundled so the helpers stay within the arg
/// cap. All `Copy` (signals are handles), so this is cheap to pass by value.
#[derive(Clone, Copy)]
struct Surface {
    lines: Signal<Vec<String>>,
    caret: Signal<(usize, usize)>,
    highlights: Option<Signal<Vec<Vec<HlSpan>>>>,
    /// Selection anchor (head is `caret`); `None`/unbound is a bare caret.
    selection: Option<SelectionAnchor>,
    /// Per-line change marks; `None` = no gutter at all.
    gutter: Option<GutterMarks>,
    /// Diagnostic spans painted as severity-tinted bands; `None` = none bound.
    diagnostics: Option<DiagSpans>,
    /// First visible row (`derived_eq` of the scroll offset).
    start: Signal<usize>,
    /// The viewport's published `(w, h)` — height drives the render budget.
    viewport: Signal<(f32, f32)>,
}

/// The materialized slice of the document to render: which row it starts at, how many
/// rows, and the row height. Keeps [`render::rows`] within the arg cap.
#[derive(Clone, Copy)]
struct Window {
    start: usize,
    budget: usize,
    row_h: f32,
}

impl IntoNode for CodeEditBuilder {
    fn build(self, cx: &mut Cx) -> NodeId {
        self.into_node().build(cx)
    }
}
