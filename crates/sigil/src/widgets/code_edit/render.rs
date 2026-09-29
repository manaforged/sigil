use crate::kernel::{NodeBuilder, TextBuilder, container, text};
use crate::layout::{AlignItems, JustifyContent, LayoutBuilderExt, LayoutStyle};
use crate::style::{NodeStyleExt, Style, Token};
use crate::widgets::grapheme::{col_bytes, col_count};

use super::{
    CARET_W, CHANGE_BAR_W, ChangeKind, DIGIT_W, DiagSeverity, DiagSpan, GUTTER_PAD, HlSpan,
    RUN_CLASS, SELECTION, SelRange, Window,
};

type DiagRange = (usize, usize, DiagSeverity);

#[derive(Clone, Copy)]
pub(super) struct RowCtx<'a> {
    pub(super) caret: (usize, usize),
    pub(super) caret_shown: bool,
    pub(super) selection: Option<SelRange>,
    pub(super) gutter: Option<GutterCtx<'a>>,
    pub(super) diagnostics: &'a [DiagSpan],
}

#[derive(Clone, Copy)]
pub(super) struct GutterCtx<'a> {
    pub(super) marks: &'a [Option<ChangeKind>],
    pub(super) width: f32,
}

#[derive(Clone, Copy)]
struct RowDecor {
    caret_col: Option<usize>,
    sel_cols: Option<(usize, usize)>,
    gutter: Option<RowGutter>,
}

#[derive(Clone, Copy)]
struct RowGutter {
    number: usize,
    mark: Option<ChangeKind>,
    width: f32,
}

pub(super) fn gutter_width(total: usize) -> f32 {
    let digits = total.max(1).to_string().len();
    #[allow(
        clippy::cast_precision_loss,
        reason = "pixel and digit-count magnitudes are bounded; sub-unit precision is irrelevant at render scale"
    )]
    let numbers = digits as f32 * DIGIT_W;
    CHANGE_BAR_W + GUTTER_PAD + numbers + GUTTER_PAD
}

pub(super) fn order(a: (usize, usize), b: (usize, usize)) -> SelRange {
    if a <= b { (a, b) } else { (b, a) }
}

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

fn row_gutter(ctx: &RowCtx<'_>, line: usize) -> Option<RowGutter> {
    let g = ctx.gutter.as_ref()?;
    Some(RowGutter {
        number: line + 1,
        mark: g.marks.get(line).copied().flatten(),
        width: g.width,
    })
}

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
        let decor = RowDecor {
            caret_col: ctx.caret_shown.then_some(0),
            sel_cols: None,
            gutter: row_gutter(&ctx, 0),
        };
        return vec![styled_row("", &[], decor, &[], row_h)];
    }
    let total = lines.len();
    let caret_line = caret_line.min(total - 1);
    let start = start.min(total - 1);
    let end = (start + budget).min(total);
    let mut out: Vec<NodeBuilder> = Vec::with_capacity(end - start);
    for (offset, line) in lines.iter().skip(start).take(end - start).enumerate() {
        let i = start + offset;
        let spans = highlights.get(i).map_or(&[][..], Vec::as_slice);
        let decor = RowDecor {
            caret_col: (ctx.caret_shown && i == caret_line).then_some(caret_col),
            sel_cols: ctx
                .selection
                .and_then(|(a, b)| selected_cols(i, a, b, col_count(line))),
            gutter: row_gutter(&ctx, i),
        };
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

fn row_bounds(n: usize, spans: &[HlSpan], decor: RowDecor, diags: &[DiagRange]) -> Vec<usize> {
    let mut bounds: Vec<usize> = Vec::with_capacity(spans.len() * 2 + diags.len() * 2 + 5);
    bounds.push(0);
    bounds.push(n);
    for s in spans {
        bounds.push(s.start.min(n));
        bounds.push(s.end.min(n));
    }
    if let Some(c) = decor.caret_col {
        bounds.push(c.min(n));
    }
    if let Some((lo, hi)) = decor.sel_cols {
        bounds.push(lo.min(n));
        bounds.push(hi.min(n));
    }
    for (start, end, _) in diags {
        bounds.push((*start).min(n));
        bounds.push((*end).min(n));
    }
    bounds.sort_unstable();
    bounds.dedup();
    bounds
}

fn styled_row(
    line: &str,
    spans: &[HlSpan],
    decor: RowDecor,
    diags: &[DiagRange],
    row_h: f32,
) -> NodeBuilder {
    let n = col_count(line);
    let caret_col = decor.caret_col.map(|c| c.min(n));
    let sel_cols = decor.sel_cols;
    let bounds = row_bounds(n, spans, decor, diags);
    let bytes = col_bytes(line, &bounds);
    let mut row = row_container(row_h);
    if let Some(g) = decor.gutter {
        row = row.child(gutter_cell(g, row_h));
    }
    for (pair, span) in bounds.windows(2).zip(bytes.windows(2)) {
        let (&[a, _], &[lo_byte, hi_byte]) = (pair, span) else {
            continue;
        };
        if caret_col == Some(a) {
            row = row.child(caret_block(row_h));
        }
        let token = spans
            .iter()
            .find(|s| s.start <= a && a < s.end)
            .map(|s| s.token);
        let seg = line[lo_byte..hi_byte].to_owned();
        if sel_cols.is_some_and(|(lo, hi)| lo <= a && a < hi) {
            row = row.child(selected_run(seg, token, row_h));
        } else if let Some(severity) = worst_diag_at(a, diags) {
            row = row.child(diag_run(seg, token, severity, row_h));
        } else {
            row = row.child(run_text(seg, token));
        }
    }
    if caret_col == Some(n) {
        row = row.child(caret_block(row_h));
    }
    row
}

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

fn change_bar(mark: Option<ChangeKind>, row_h: f32) -> NodeBuilder {
    let bar = container().layout(LayoutStyle::flex_row().width(CHANGE_BAR_W).height(row_h));
    match mark {
        Some(kind) => bar.class(kind.bar_token().bg()),
        None => bar,
    }
}

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
                .class("text-sm font-mono")
                .class(Token::OnSurfaceMuted.text())
                .layout(LayoutStyle::block().shrink(0.0)),
        )
}

fn worst_diag_at(a: usize, diags: &[DiagRange]) -> Option<DiagSeverity> {
    diags
        .iter()
        .filter(|(start, end, _)| *start <= a && a < *end)
        .map(|(_, _, severity)| *severity)
        .max()
}

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
        .class(severity.tint_token().bg())
        .child(run_text(seg, token))
}

fn selected_run(seg: String, token: Option<&'static str>, row_h: f32) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_row()
                .height(row_h)
                .shrink(0.0)
                .align_items(AlignItems::Center),
        )
        .class(SELECTION.bg())
        .child(run_text(seg, token))
}

fn run_text(seg: String, token: Option<&'static str>) -> TextBuilder {
    text(seg)
        .class(RUN_CLASS)
        .style(Style::new().text_theme(token.unwrap_or(Token::OnSurface.name())))
        .layout(LayoutStyle::block().shrink(0.0))
}

fn row_container(row_h: f32) -> NodeBuilder {
    container().layout(
        LayoutStyle::flex_row()
            .height(row_h)
            .shrink(0.0)
            .align_items(AlignItems::Center),
    )
}

fn caret_block(row_h: f32) -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_row().width(CARET_W).height(row_h * 0.8))
        .class(Token::OnSurface.bg())
}
