//! Window tree matching GPUI `examples/data_table.rs`.
//!
//! Status line + clipped 24-column header + `virtual_list` body
//! (scroll reuses keyed rows; quote ticks update cell text in place)
//! + 8×100 thumb.

use std::ops::Range;
use std::rc::Rc;

use sigil::prelude::*;
use sigil::widgets::Axis;

use crate::fields::{
    FIELDS, Field, FieldKind, TABLE_WIDTH, VIEWPORT_HEIGHT, cell_text, header_label,
};
use crate::quote::{Quote, ROW_HEIGHT, TOTAL_ITEMS, generate};

pub const WINDOW_W: f32 = 1280.0;
pub const WINDOW_H: f32 = 1000.0;
const PAD: f32 = 16.0;
const GAP: f32 = 8.0;
const STATUS_H: f32 = 20.0;
const HEADER_H: f32 = 28.0;
const ROW_H: f32 = ROW_HEIGHT;
const TABLE_W: f32 = TABLE_WIDTH;
const TABLE_H: f32 = WINDOW_H - PAD * 2.0 - GAP - STATUS_H;
pub const VIEWPORT_H: f32 = VIEWPORT_HEIGHT;
const _: () = assert!((TABLE_H - HEADER_H - VIEWPORT_HEIGHT).abs() < 0.01);
const THUMB_W: f32 = 8.0;
const THUMB_H: f32 = 100.0;
const CONTENT_H: f32 = 10_000.0 * ROW_H;
const _: () = assert!(TOTAL_ITEMS == 10_000);

const fn rgb(r: u8, g: u8, b: u8) -> Rgba8 {
    Rgba8 { r, g, b, a: 255 }
}

const WHITE: Rgba8 = rgb(255, 255, 255);
const ZEBRA: Rgba8 = rgb(250, 250, 250);
const BORDER: Rgba8 = rgb(224, 224, 224);
const HEADER_BG: Rgba8 = rgb(240, 240, 240);
const HEADER_FG: Rgba8 = rgb(85, 85, 85);
const THUMB: Rgba8 = rgb(192, 192, 192);
const THUMB_HOVER: Rgba8 = rgb(160, 160, 160);
const GREEN: Rgba8 = rgb(0, 128, 0);
const RED: Rgba8 = rgb(255, 0, 0);

/// Build the full data-table tree.
#[must_use]
pub fn app_tree() -> NodeBuilder {
    let items = signal(Rc::new(generate(TOTAL_ITEMS)));
    let scroll_y = signal(0.0_f32);
    container()
        .role(Role::Main)
        .label("Sigil data table")
        .layout(
            LayoutStyle::flex_column()
                .fixed_size(WINDOW_W, WINDOW_H)
                .gap(GAP)
                .padding(PAD),
        )
        .style(Style::new().bg(WHITE))
        .class(classes!("text-sm"))
        .child(status_line(scroll_y))
        .child(table_frame(items, scroll_y))
}

/// Visible row range at `scroll_y`, same formula as `virtual_list`.
///
/// One viewport of rows on each side of the geometric page.
#[must_use]
pub fn visible_range(scroll_y: f32, total: usize) -> Range<usize> {
    if total == 0 {
        return 0..0;
    }
    let row_h = ROW_H.max(1.0);
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        clippy::cast_sign_loss
    )]
    let page = ((VIEWPORT_H / row_h).ceil() as usize).max(1);
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let vis_first = (scroll_y / row_h).floor().max(0.0) as usize;
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        clippy::cast_sign_loss
    )]
    let vis_end = ((scroll_y + VIEWPORT_H) / row_h).ceil() as usize;
    let first = vis_first.saturating_sub(page).min(total);
    first..vis_end.saturating_add(page).min(total)
}

fn status_line(scroll_y: Signal<f32>) -> NodeBuilder {
    let caption = derived(move || {
        let range = visible_range(scroll_y.get(), TOTAL_ITEMS);
        format!("Total {TOTAL_ITEMS} items, visible range: {range:?}")
    });
    container()
        .layout(LayoutStyle::block().height(STATUS_H))
        .child(text(caption).nowrap().class(classes!("text-sm")))
}

fn table_frame(items: Signal<Rc<Vec<Quote>>>, scroll_y: Signal<f32>) -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_column().fixed_size(TABLE_W, TABLE_H))
        .class(classes!("overflow-hidden rounded-sm"))
        .style(
            Style::new()
                .border_width(1)
                .border_color(ColorAtom::Rgba8(BORDER)),
        )
        .child(header_row())
        .child(body(items, scroll_y))
}

fn header_row() -> NodeBuilder {
    let mut row = container()
        .layout(
            LayoutStyle::flex_row()
                .width(TABLE_W)
                .height(HEADER_H)
                .shrink(0.0),
        )
        .class(classes!("px-2 overflow-hidden text-xs"))
        .style(
            Style::new()
                .bg(HEADER_BG)
                .text_color(ColorAtom::Rgba8(HEADER_FG)),
        );
    for field in FIELDS {
        row = row.child(header_cell(field));
    }
    row
}

fn header_cell(field: Field) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::block()
                .width(field.width)
                .height(HEADER_H)
                .shrink(0.0),
        )
        .class(classes!("px-1 overflow-hidden"))
        .child(
            text(header_label(field.key))
                .nowrap()
                .class(classes!("text-xs"))
                .style(Style::new().text_color(ColorAtom::Rgba8(HEADER_FG))),
        )
}

fn body(items: Signal<Rc<Vec<Quote>>>, scroll_y: Signal<f32>) -> NodeBuilder {
    container()
        .layout(LayoutStyle::block().fixed_size(TABLE_W, VIEWPORT_H))
        .class(classes!("overflow-hidden"))
        .child(quote_list(items, scroll_y))
        .child(scrollbar(scroll_y))
}

/// Windowed body used by the live window and the Sigil bench runner.
#[must_use]
pub fn quote_list(items: Signal<Rc<Vec<Quote>>>, scroll_y: Signal<f32>) -> impl IntoNode {
    virtual_list(items, move |index, quote| table_row(index, quote, items))
        .row_height(ROW_H)
        .viewport_height(VIEWPORT_H)
        .viewport_width(TABLE_W)
        .scroll_bind(scroll_y)
        .keyboard(false)
}

/// One 24-column quote row — the production cell tree the bench times.
#[must_use]
pub fn table_row(index: usize, quote: &Quote, book: Signal<Rc<Vec<Quote>>>) -> NodeBuilder {
    let tint = if quote.up() { GREEN } else { RED };
    let bg = if index.is_multiple_of(2) {
        WHITE
    } else {
        ZEBRA
    };
    let mut row = container()
        .layout(LayoutStyle::flex_row().height(ROW_H).shrink(0.0))
        .class(classes!("px-2 overflow-hidden"))
        .style(
            Style::new()
                .bg(bg)
                .border_bottom(1)
                .border_color(ColorAtom::Rgba8(BORDER)),
        );
    for field in FIELDS {
        let value = cell_source(book, index, quote, field.kind);
        let color = field.tint.then_some(tint);
        row = row.child(data_cell(field.width, value, color));
    }
    row
}

fn cell_source(
    book: Signal<Rc<Vec<Quote>>>,
    index: usize,
    quote: &Quote,
    kind: FieldKind,
) -> TextContent {
    if kind.live() {
        TextContent::compute(move || {
            book.get()
                .get(index)
                .map_or_else(String::new, |row| cell_text(index, row, kind))
        })
    } else {
        TextContent::Static(cell_text(index, quote, kind))
    }
}

fn data_cell(width: f32, value: TextContent, color: Option<Rgba8>) -> NodeBuilder {
    let mut label = text(value)
        .nowrap()
        .class(classes!("text-sm"))
        .layout(LayoutStyle::block().width(width).height(ROW_H));
    if let Some(color) = color {
        label = label.style(Style::new().text_color(ColorAtom::Rgba8(color)));
    }
    container()
        .layout(LayoutStyle::block().width(width).height(ROW_H).shrink(0.0))
        .class(classes!("px-1 overflow-hidden"))
        .child(label)
}

fn scrollbar(scroll_y: Signal<f32>) -> NodeBuilder {
    let pos = derived(move || (TABLE_W - THUMB_W - 4.0, thumb_top(scroll_y.get())));
    container()
        .layout(LayoutStyle::block().fixed_size(THUMB_W, THUMB_H))
        .position_reactive(pos)
        .child(thumb(scroll_y))
}

fn thumb_top(scroll_y: f32) -> f32 {
    let content = CONTENT_H;
    if content <= 0.0 {
        return 4.0;
    }
    let pct = (scroll_y / content).clamp(0.0, 1.0);
    let max_top = (VIEWPORT_H - THUMB_H - 4.0).max(4.0);
    (VIEWPORT_H * pct).clamp(4.0, max_top)
}

fn thumb(scroll_y: Signal<f32>) -> NodeBuilder {
    let node = interactive(
        container()
            .layout(LayoutStyle::block().fixed_size(THUMB_W, THUMB_H))
            .class(classes!("rounded-lg"))
            .style(Style::new().bg(THUMB)),
    )
    .hover(|style| style.bg(THUMB_HOVER))
    .into_node_builder();
    axis_drag(
        node,
        Axis::Y,
        move || scroll_y.peek(),
        move |start, delta| {
            scroll_y.set(clamped_scroll(start, delta));
        },
        || {},
    )
}

fn clamped_scroll(start: f32, delta: f32) -> f32 {
    let content = CONTENT_H;
    let max_scroll = (content - VIEWPORT_H).max(0.0);
    let scale = if VIEWPORT_H > 0.0 {
        content / VIEWPORT_H
    } else {
        1.0
    };
    delta.mul_add(scale, start).clamp(0.0, max_scroll)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visible_range_at_top_covers_first_window() {
        let range = visible_range(0.0, TOTAL_ITEMS);
        assert_eq!(range.start, 0);
        // 38 in view + 38 overscan below.
        assert_eq!(range.end, 76);
    }

    #[test]
    fn visible_range_empty_corpus_is_empty() {
        assert_eq!(visible_range(0.0, 0), 0..0);
    }
}
