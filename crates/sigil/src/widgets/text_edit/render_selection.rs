use crate::kernel::{NodeBuilder, container, fragment};
use crate::layout::prelude::{LayoutBuilderExt, LayoutStyle, Position};
use crate::layout::with_text_geometry;
use crate::style::{NodeStyleExt, StyleAtom, TextStyle};

use super::{TextEditRuntime, caret_line_col, line_height, nth_line};

static SELECTION_ATOMS: &[StyleAtom] = &[StyleAtom::BackgroundColor(
    crate::style::ColorAtom::Rgba8(crate::style::Rgba8 {
        r: 56,
        g: 118,
        b: 224,
        a: 130,
    }),
)];
const NEWLINE_SEL_W: f32 = 6.0;

pub(super) fn selection_layer(runtime: &TextEditRuntime) -> NodeBuilder {
    let value = runtime.value;
    let caret = runtime.caret;
    let selection = runtime.selection;
    fragment()
        .layout(LayoutStyle::block())
        .children_reactive(move || {
            let buffer = value.get();
            let Some(anchor) = selection.get() else {
                return Vec::new();
            };
            let caret = caret.get().min(buffer.len());
            let anchor = anchor.min(buffer.len());
            let (lo, hi) = (anchor.min(caret), anchor.max(caret));
            selection_rects(&buffer, lo, hi)
        })
}

fn selection_rects(buffer: &str, lo: usize, hi: usize) -> Vec<NodeBuilder> {
    if lo >= hi {
        return Vec::new();
    }
    let (lo_line, lo_col) = caret_line_col(buffer, lo);
    let (hi_line, hi_col) = caret_line_col(buffer, hi);
    let line_h = line_height();
    (lo_line..=hi_line)
        .filter_map(|line| {
            let start_col = if line == lo_line { lo_col } else { 0 };
            let end_col = if line == hi_line { hi_col } else { usize::MAX };
            line_highlight(buffer, line, (start_col, end_col), hi_line, line_h)
        })
        .collect()
}

fn line_highlight(
    buffer: &str,
    line: usize,
    cols: (usize, usize),
    hi_line: usize,
    line_h: f32,
) -> Option<NodeBuilder> {
    let (start_col, end_col) = cols;
    let line_text = nth_line(buffer, line);
    let (x0, x1) = with_text_geometry(|engine| {
        let style = TextStyle::default();
        let a = engine
            .caret(line_text, &style, start_col.min(line_text.len()))
            .x;
        let b = engine
            .caret(line_text, &style, end_col.min(line_text.len()))
            .x;
        (a, b)
    })?;
    let min_w = if line < hi_line { NEWLINE_SEL_W } else { 0.0 };
    let width = (x1 - x0).max(min_w);
    if width <= 0.0 {
        return None;
    }
    #[allow(
        clippy::cast_precision_loss,
        reason = "pixel and char-count magnitudes are bounded; sub-unit precision is irrelevant at render scale"
    )]
    let y = line as f32 * line_h;
    Some(
        container()
            .hittable(false)
            .layout(
                LayoutStyle::block()
                    .position(Position::Absolute)
                    .inset_left(x0)
                    .inset_top(y)
                    .width(width)
                    .height(line_h),
            )
            .class(SELECTION_ATOMS),
    )
}
