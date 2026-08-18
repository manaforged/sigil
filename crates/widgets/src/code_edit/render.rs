//! Row painting: the materialized window of rows and the cut-and-rebuild of a single
//! line into colored runs, the caret block, the selection band, and the optional gutter
//! (line number + change bar). Pure given its inputs — `scroll` decides *which* rows,
//! this module decides how each one looks.

use sigil_kernel::{NodeBuilder, TextBuilder, container, text};
use sigil_layout::prelude::{AlignItems, JustifyContent, LayoutBuilderExt, LayoutStyle};
use sigil_style::{NodeStyleExt, Style};

use super::{
    CARET_W, CHANGE_BAR_W, ChangeKind, DIGIT_W, DiagSeverity, DiagSpan, GUTTER_PAD, HlSpan,
    RUN_CLASS, SELECTION_CLASS, SelRange, Window,
};

/// A diagnosed char range on a single row: `[start, end)` at `severity`.
type DiagRange = (usize, usize, DiagSeverity);

/// The selection + gutter + diagnostic inputs [`rows`] needs beyond the buffer + window,
/// bundled to keep it within the arg cap. Borrows the per-frame slices.
#[derive(Clone, Copy)]
pub(super) struct RowCtx<'a> {
    pub(super) caret: (usize, usize),
    pub(super) selection: Option<SelRange>,
    pub(super) gutter: Option<GutterCtx<'a>>,
    pub(super) diagnostics: &'a [DiagSpan],
}

/// The gutter inputs for a frame: the per-line change marks and the computed column width
/// (so every row's number column aligns regardless of the visible window).
#[derive(Clone, Copy)]
pub(super) struct GutterCtx<'a> {
    pub(super) marks: &'a [Option<ChangeKind>],
    pub(super) width: f32,
}

/// Per-row decorations layered over the glyph runs: caret, selection band, and an optional
/// gutter cell. Bundled to keep [`styled_row`] within the arg cap.
#[derive(Clone, Copy)]
struct RowDecor {
    caret_col: Option<usize>,
    sel_cols: Option<(usize, usize)>,
    gutter: Option<RowGutter>,
}

/// One row's gutter: its 1-based line number, its change bar (if any), and the shared
/// column width.
#[derive(Clone, Copy)]
struct RowGutter {
    number: usize,
    mark: Option<ChangeKind>,
    width: f32,
}

/// The gutter column width for a document of `total` lines: change bar + padded
/// number column sized to the widest line number. At least one digit so an empty/1-line
/// file still gets a stable column.
pub(super) fn gutter_width(total: usize) -> f32 {
    let digits = total.max(1).to_string().len();
    #[allow(clippy::cast_precision_loss)]
    let numbers = digits as f32 * DIGIT_W;
    CHANGE_BAR_W + GUTTER_PAD + numbers + GUTTER_PAD
}

/// Order two `(line, col)` positions so the first is `<=` the second.
pub(super) fn order(a: (usize, usize), b: (usize, usize)) -> SelRange {
    if a <= b { (a, b) } else { (b, a) }
}

/// The selected `[lo, hi)` char range on row `i`, or `None` if the row is outside the
/// selection (or only its zero-width edge touches it). For a multi-line selection the
/// interior/edge lines extend to the line's end.
fn selected_cols(
    i: usize,
    start: (usize, usize),
    end: (usize, usize),
    line_len: usize,
) -> Option<(usize, usize)> {
    if i < start.0 || i > end.0 {
        return None;
    }
    let lo = if i == start.0 {
        start.1.min(line_len)
    } else {
        0
    };
    let hi = if i == end.0 {
        end.1.min(line_len)
    } else {
        line_len
    };
    (lo < hi).then_some((lo, hi))
}

/// This row's gutter cell, or `None` when the surface has no gutter. The change mark is
/// looked up per line (a short/empty marks vec just means later lines are unchanged).
fn row_gutter(ctx: &RowCtx<'_>, line: usize) -> Option<RowGutter> {
    let g = ctx.gutter.as_ref()?;
    Some(RowGutter {
        number: line + 1,
        mark: g.marks.get(line).copied().flatten(),
        width: g.width,
    })
}

/// The materialized window: [`Window::budget`] rows starting at [`Window::start`]. Only
/// these become nodes, so document length no longer drives per-frame node count — the
/// virtualization that fixed the all-rows-re-solve stall. Rows render at the top of the
/// (clipping) viewport; which lines show is chosen entirely by `start`.
pub(super) fn rows(
    lines: &[String],
    highlights: &[Vec<HlSpan>],
    win: Window,
    ctx: RowCtx<'_>,
) -> Vec<NodeBuilder> {
    let Window {
        start,
        budget,
        row_h,
    } = win;
    let (caret_line, caret_col) = ctx.caret;
    if lines.is_empty() {
        // An empty document still shows a caret (and a "1" gutter row) on its single row.
        let decor = RowDecor {
            caret_col: Some(0),
            sel_cols: None,
            gutter: row_gutter(&ctx, 0),
        };
        return vec![styled_row("", &[], decor, &[], row_h)];
    }
    let total = lines.len();
    // The caret line can momentarily point past the end (a stale caret after the doc
    // shrank); clamp it onto the last real row so it still draws instead of vanishing.
    let caret_line = caret_line.min(total - 1);
    let start = start.min(total - 1);
    let end = (start + budget).min(total);
    let mut out: Vec<NodeBuilder> = Vec::with_capacity(end - start);
    for (offset, line) in lines.iter().skip(start).take(end - start).enumerate() {
        let i = start + offset;
        let spans = highlights.get(i).map_or(&[][..], Vec::as_slice);
        let decor = RowDecor {
            caret_col: (i == caret_line).then_some(caret_col),
            sel_cols: ctx
                .selection
                .and_then(|(a, b)| selected_cols(i, a, b, line.chars().count())),
            gutter: row_gutter(&ctx, i),
        };
        // This row's diagnosed ranges (the slice is usually tiny — a handful per file).
        let diags: Vec<DiagRange> = ctx
            .diagnostics
            .iter()
            .filter(|d| d.line == i)
            .map(|d| (d.start, d.end, d.severity))
            .collect();
        out.push(styled_row(line, spans, decor, &diags, row_h));
    }
    out
}

/// One line rendered as colored runs, with the caret block inserted at `caret_col`
/// (if any) and an optional gutter cell prepended. Runs are cut at every span boundary and
/// at the caret, so the caret sits exactly between glyphs and each run carries its color.
fn styled_row(
    line: &str,
    spans: &[HlSpan],
    decor: RowDecor,
    diags: &[DiagRange],
    row_h: f32,
) -> NodeBuilder {
    let caret_col = decor.caret_col;
    let sel_cols = decor.sel_cols;
    let chars: Vec<char> = line.chars().collect();
    let n = chars.len();
    let mut bounds: Vec<usize> = Vec::with_capacity(spans.len() * 2 + diags.len() * 2 + 5);
    bounds.push(0);
    bounds.push(n);
    for s in spans {
        bounds.push(s.start.min(n));
        bounds.push(s.end.min(n));
    }
    if let Some(c) = caret_col {
        bounds.push(c.min(n));
    }
    // Cut at the selection edges too, so a run is wholly inside or outside the band.
    if let Some((lo, hi)) = sel_cols {
        bounds.push(lo.min(n));
        bounds.push(hi.min(n));
    }
    // Cut at every diagnostic edge so a run is wholly inside or outside a tint.
    for (start, end, _) in diags {
        bounds.push(start.min(&n).to_owned());
        bounds.push(end.min(&n).to_owned());
    }
    bounds.sort_unstable();
    bounds.dedup();
    let mut row = row_container(row_h);
    // The gutter is the row's first child, so the text starts at a stable left margin.
    if let Some(g) = decor.gutter {
        row = row.child(gutter_cell(g, row_h));
    }
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
        // Selection wins over a diagnostic tint (both are background bands); a diagnosed
        // run paints the most severe overlapping severity behind the glyphs.
        if sel_cols.is_some_and(|(lo, hi)| lo <= a && a < hi) {
            row = row.child(selected_run(seg, token, row_h));
        } else if let Some(severity) = worst_diag_at(a, diags) {
            row = row.child(diag_run(seg, token, severity, row_h));
        } else {
            row = row.child(run_text(seg, token));
        }
    }
    // A caret at end-of-line (or on an empty line) sits after the last run.
    if caret_col == Some(n) {
        row = row.child(caret_block(row_h));
    }
    row
}

/// The gutter cell: a fixed-width change bar at the far left, then the right-aligned line
/// number filling the rest of the column so all numbers share a right edge.
fn gutter_cell(g: RowGutter, row_h: f32) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_row()
                .width(g.width)
                .height(row_h)
                .shrink(0.0)
                .align_items(AlignItems::Center),
        )
        .child(change_bar(g.mark, row_h))
        .child(number_text(g.number, row_h))
}

/// The change bar: a full-height colored strip when the line changed, else an empty strip
/// of the same width so the numbers don't shift between changed and unchanged rows.
fn change_bar(mark: Option<ChangeKind>, row_h: f32) -> NodeBuilder {
    let bar = container().layout(LayoutStyle::flex_row().width(CHANGE_BAR_W).height(row_h));
    match mark {
        Some(kind) => bar.class(kind.bar_class()),
        None => bar,
    }
}

/// The right-aligned, dimmed line number filling the column to the right of the change bar.
fn number_text(number: usize, row_h: f32) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_row()
                .grow(1.0)
                .height(row_h)
                .align_items(AlignItems::Center)
                .justify_content(JustifyContent::End)
                .padding_x(GUTTER_PAD),
        )
        .child(
            text(number.to_string())
                .class("text-sm font-mono text-muted-foreground")
                .layout(LayoutStyle::block().shrink(0.0)),
        )
}

/// The most severe diagnostic covering char column `a`, or `None` if none do. `Ord` on
/// [`DiagSeverity`] (declared least-to-most severe) makes `max` pick the worst.
fn worst_diag_at(a: usize, diags: &[DiagRange]) -> Option<DiagSeverity> {
    diags
        .iter()
        .filter(|(start, end, _)| *start <= a && a < *end)
        .map(|(_, _, severity)| *severity)
        .max()
}

/// A glyph run under a diagnostic: [`run_text`] over a faint severity-colored band (the
/// underline-substitute until the renderer grows a wavy primitive). Mirrors [`selected_run`].
fn diag_run(
    seg: String,
    token: Option<&'static str>,
    severity: DiagSeverity,
    row_h: f32,
) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_row()
                .height(row_h)
                .shrink(0.0)
                .align_items(AlignItems::Center),
        )
        .class(severity.tint_class())
        .child(run_text(seg, token))
}

/// A glyph run inside a selection: the [`run_text`] over a selection-tinted band that fills
/// the row height, so a multi-glyph selection reads as one continuous highlight.
fn selected_run(seg: String, token: Option<&'static str>, row_h: f32) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_row()
                .height(row_h)
                .shrink(0.0)
                .align_items(AlignItems::Center),
        )
        .class(SELECTION_CLASS)
        .child(run_text(seg, token))
}

/// A glyph run colored by a theme token, defaulting to the editor foreground.
///
/// `shrink(0.0)` is the no-wrap guarantee: a run lives in the row's `flex_row`, and a
/// line wider than the pane would otherwise shrink the run to the available width — at
/// which point PAINT re-wraps that run to fit, drawing extra visual lines that overflow
/// the rigid 18px row and bleed onto the next (the residual "long lines overlap" bug
/// that survives row-level `shrink(0)`). Pinning the run keeps its full intrinsic width
/// so it paints as one line and the container's `overflow_x: Hidden` clips it.
fn run_text(seg: String, token: Option<&'static str>) -> TextBuilder {
    text(seg)
        .class(RUN_CLASS)
        .style(Style::new().text_theme(token.unwrap_or("foreground")))
        .layout(LayoutStyle::block().shrink(0.0))
}

/// Shared row chrome: a fixed-height flex row so caret rows and plain rows align.
///
/// `shrink(0.0)` is load-bearing: the rows live in a `flex_column` bounded by the editor
/// pane height, and a file taller than the pane overflows it. Without rigid rows, flex's
/// default shrink compresses every row below `row_h` to fit the column — squishing the
/// lines until their text overlaps, worse the longer the file. Rigid rows keep their full
/// height and let the column clip the overflow instead.
fn row_container(row_h: f32) -> NodeBuilder {
    container().layout(
        LayoutStyle::flex_row()
            .height(row_h)
            .shrink(0.0)
            .align_items(AlignItems::Center),
    )
}

/// The caret bar (static for now; a blink signal is a follow-up).
fn caret_block(row_h: f32) -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_row().width(CARET_W).height(row_h * 0.8))
        .class("bg-foreground")
}
