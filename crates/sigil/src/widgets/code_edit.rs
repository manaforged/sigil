//! `code_edit` — a multi-line, **no-wrap** code surface.
//!
//! Renders an arbitrary buffer as one monospace row per line (long lines clip rather
//! than wrap). Each row is painted as colored runs from the optional per-line
//! [`HlSpan`]s; the caret is a block drawn *between glyphs* by cutting the row at the
//! caret column, so the text engine positions it exactly. The widget owns **no buffer**
//! — it renders the `lines` / `caret` / `highlights` signals and forwards keys to
//! the `key` handler, so the consumer keeps one source of truth (e.g. an engine rope) elsewhere.

use std::rc::Rc;

use events::prelude::{KeyEvent, NodeEventsExt};
use kernel::{Cx, IntoNode, NodeBuilder, NodeId, Role, TextBuilder, container, text};
use layout::prelude::{AlignItems, LayoutBuilderExt, LayoutStyle, Overflow};
use reactive::Signal;
use style::{NodeStyleExt, Style};

/// Default row height in logical px — `text-sm` mono (14px × ~1.3 leading).
const ROW_H: f32 = 18.0;
/// Caret bar width in logical px.
const CARET_W: f32 = 2.0;
/// Size + font for every glyph run. Color is applied per-run via a theme token (so a
/// syntax span can recolor without fighting a `text-*` color class).
const RUN_CLASS: &str = "text-sm font-mono";
/// Upper bound on rendered rows. One node per visible line is fine for a screenful
/// but a 100k-line file would build 100k nodes per keystroke, so the surface renders a
/// caret-anchored window of at most this many rows with explicit "more lines" markers
/// above/below. True windowed virtualization (a scroll offset) is the follow-up.
const MAX_ROWS: usize = 2_000;

type KeyHandler = Rc<dyn Fn(&mut KeyEvent) + 'static>;

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

/// Builder returned by [`code_edit`].
pub struct CodeEditBuilder {
    lines: Signal<Vec<String>>,
    caret: Signal<(usize, usize)>,
    highlights: Option<Signal<Vec<Vec<HlSpan>>>>,
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
        on_key: None,
        row_h: ROW_H,
    }
}

impl CodeEditBuilder {
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

    fn into_node(self) -> NodeBuilder {
        let lines = self.lines;
        let caret = self.caret;
        let highlights = self.highlights;
        let row_h = self.row_h;
        // Only trap Tab when an editing handler is installed. A read-only code surface
        // (no `on_key`) must NOT swallow Tab — that would strand focus traversal.
        let traps_tab = self.on_key.is_some();
        let on_key = self.on_key.unwrap_or_else(|| Rc::new(|_| {}));
        container()
            .role(Role::TextInput)
            .label("Code editor")
            .focusable(true)
            // Receive a plain Tab to insert an indent instead of letting the router
            // move focus (Shift+Tab still escapes, so this is no keyboard trap).
            .traps_tab(traps_tab)
            // The box fills its parent (`w-full h-full`) and **clips** overflow, while
            // `align_items(Start)` keeps each row at its content (max-content) width. So a
            // long line overflows the box and is clipped — it never soft-wraps onto the
            // next row, AND the widget's hit-box stays inside its pane instead of growing
            // to the longest line's width and stealing clicks from neighbouring panes.
            .class("w-full h-full")
            .layout(
                LayoutStyle::flex_column()
                    .align_items(AlignItems::Start)
                    .overflow_x(Overflow::Hidden)
                    .overflow_y(Overflow::Hidden),
            )
            .children_reactive(move || {
                let hl = highlights.map(|s| s.get()).unwrap_or_default();
                rows(&lines.get(), &hl, caret.get(), row_h)
            })
            .key_any(move |event| on_key(event))
    }
}

impl IntoNode for CodeEditBuilder {
    fn build(self, cx: &mut Cx) -> NodeId {
        self.into_node().build(cx)
    }
}

/// The caret-anchored window of rows, with overflow markers when the doc exceeds the cap.
fn rows(
    lines: &[String],
    highlights: &[Vec<HlSpan>],
    caret: (usize, usize),
    row_h: f32,
) -> Vec<NodeBuilder> {
    let (caret_line, caret_col) = caret;
    if lines.is_empty() {
        // An empty document still shows a caret on its single empty row.
        return vec![styled_row("", &[], Some(0), row_h)];
    }
    // Clamp the caret to a row that actually exists (the signals can momentarily
    // disagree), then render a window that ALWAYS contains it so it never hides.
    let total = lines.len();
    let caret_line = caret_line.min(total - 1);
    let start = if total <= MAX_ROWS {
        0
    } else {
        caret_line
            .saturating_sub(MAX_ROWS / 2)
            .min(total - MAX_ROWS)
    };
    let end = (start + MAX_ROWS).min(total);
    let mut rows: Vec<NodeBuilder> = Vec::with_capacity(end - start + 2);
    if start > 0 {
        rows.push(marker_row(
            format!("… {start} lines above (virtual scroll pending)"),
            row_h,
        ));
    }
    for (offset, line) in lines.iter().skip(start).take(end - start).enumerate() {
        let i = start + offset;
        let spans = highlights.get(i).map_or(&[][..], Vec::as_slice);
        let row_caret = (i == caret_line).then_some(caret_col);
        rows.push(styled_row(line, spans, row_caret, row_h));
    }
    if end < total {
        rows.push(marker_row(
            format!("… {} lines below (virtual scroll pending)", total - end),
            row_h,
        ));
    }
    rows
}

/// One line rendered as colored runs, with the caret block inserted at `caret_col`
/// (if any). Runs are cut at every span boundary and at the caret, so the caret sits
/// exactly between glyphs and each run carries its span's color token.
fn styled_row(line: &str, spans: &[HlSpan], caret_col: Option<usize>, row_h: f32) -> NodeBuilder {
    let chars: Vec<char> = line.chars().collect();
    let n = chars.len();
    let mut bounds: Vec<usize> = Vec::with_capacity(spans.len() * 2 + 3);
    bounds.push(0);
    bounds.push(n);
    for s in spans {
        bounds.push(s.start.min(n));
        bounds.push(s.end.min(n));
    }
    if let Some(c) = caret_col {
        bounds.push(c.min(n));
    }
    bounds.sort_unstable();
    bounds.dedup();
    let mut row = row_container(row_h);
    for pair in bounds.windows(2) {
        let &[a, b] = pair else { continue };
        if caret_col == Some(a) {
            row = row.child(caret_block(row_h));
        }
        let token = spans
            .iter()
            .find(|s| s.start <= a && a < s.end)
            .map(|s| s.token);
        let seg: String = chars.iter().skip(a).take(b - a).collect();
        row = row.child(run_text(seg, token));
    }
    // A caret at end-of-line (or on an empty line) sits after the last run.
    if caret_col == Some(n) {
        row = row.child(caret_block(row_h));
    }
    row
}

/// A glyph run colored by a theme token, defaulting to the editor foreground.
fn run_text(seg: String, token: Option<&'static str>) -> TextBuilder {
    text(seg)
        .class(RUN_CLASS)
        .style(Style::new().text_theme(token.unwrap_or("foreground")))
}

/// A dim "N lines above/below" overflow marker row.
fn marker_row(label: String, row_h: f32) -> NodeBuilder {
    row_container(row_h).child(
        text(label)
            .class(RUN_CLASS)
            .style(Style::new().text_theme("muted-foreground")),
    )
}

/// Shared row chrome: a fixed-height flex row so caret rows and plain rows align.
fn row_container(row_h: f32) -> NodeBuilder {
    container().layout(
        LayoutStyle::flex_row()
            .height(row_h)
            .align_items(AlignItems::Center),
    )
}

/// The caret bar (static for now; a blink signal is a follow-up).
fn caret_block(row_h: f32) -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_row().width(CARET_W).height(row_h * 0.8))
        .class("bg-foreground")
}
