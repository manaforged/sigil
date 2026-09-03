use std::ops::Range;

use bench_spec::ledger::{ROW_H, VIEWPORT};
use data_table::ledger::Row;
use data_table::{FIELDS, Field, FieldKind, fields::header_label};
use gpui::{
    Context, Div, IntoElement, ParentElement, Render, Styled, Window, div, green, px, red, rgb,
    uniform_list,
};

use crate::Ledger;

const HEADER_H: f32 = 28.0;

impl Render for Ledger {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let (bg, fg) = if self.dark {
            (rgb(0x001E_1E1E), rgb(0x00EE_EEEE))
        } else {
            (rgb(0x00FF_FFFF), rgb(0x0011_1111))
        };
        div()
            .bg(bg)
            .text_color(fg)
            .text_sm()
            .size_full()
            .p_4()
            .gap_2()
            .flex()
            .flex_col()
            .child(self.toolbar())
            .child(
                div()
                    .flex()
                    .flex_col()
                    .w(px(VIEWPORT.0))
                    .h(px(VIEWPORT.1 + HEADER_H))
                    .overflow_hidden()
                    .border_1()
                    .border_color(rgb(0x00E0_E0E0))
                    .child(header())
                    .child(
                        uniform_list(
                            "rows",
                            self.order.len(),
                            cx.processor(|this, range: Range<usize>, _, _| this.rows(range)),
                        )
                        .h(px(VIEWPORT.1))
                        .track_scroll(self.scroll.clone()),
                    ),
            )
            .child(self.status())
    }
}

impl Ledger {
    fn toolbar(&self) -> Div {
        let filter = if self.filter.is_empty() {
            String::from("Filter symbol or name")
        } else {
            self.filter.clone()
        };
        let sort = self.sort.map_or_else(
            || String::from("unsorted"),
            |(kind, ascending)| {
                let dir = if ascending { "Ascending" } else { "Descending" };
                format!("sort {kind:?} {dir}")
            },
        );
        div()
            .flex()
            .flex_row()
            .h(px(32.0))
            .gap_2()
            .items_center()
            .child(
                div()
                    .w(px(260.0))
                    .h(px(28.0))
                    .px_2()
                    .border_1()
                    .border_color(rgb(0x00C0_C0C0))
                    .child(filter),
            )
            .child(div().text_xs().child(sort))
            .child(button(format!("Undo {}", self.undo.len())))
            .child(button("Theme"))
    }

    fn status(&self) -> Div {
        div().h(px(20.0)).text_xs().child(format!(
            "{} rows · {} shown · {} selected · {} ticks · {} cells moved",
            self.book.len(),
            self.order.len(),
            usize::from(self.selected.is_some()),
            self.ticks,
            self.moved
        ))
    }

    fn rows(&self, range: Range<usize>) -> Vec<Div> {
        range
            .filter_map(|ix| {
                let index = *self.order.get(ix)?;
                let row = self.row(index)?;
                Some(self.row_element(ix, index, row))
            })
            .collect()
    }

    fn row_element(&self, ix: usize, index: u32, row: &Row) -> Div {
        let zebra = if ix.is_multiple_of(2) {
            rgb(0x00FF_FFFF)
        } else {
            rgb(0x00FA_FAFA)
        };
        let bg = if self.selected == Some(index) {
            rgb(0x00DC_E8FF)
        } else {
            zebra
        };
        let tint = if row.quote.up() { green() } else { red() };
        let mut element = div()
            .flex()
            .flex_row()
            .h(px(ROW_H))
            .border_b_1()
            .border_color(rgb(0x00E0_E0E0))
            .bg(bg)
            .py_0p5()
            .px_2();
        for field in &FIELDS {
            element = element.child(self.cell(index, row, field, tint));
        }
        element
    }

    fn cell(&self, index: u32, row: &Row, field: &Field, tint: gpui::Hsla) -> Div {
        let base = div()
            .whitespace_nowrap()
            .truncate()
            .w(px(field.width))
            .px_1();
        if field.kind == FieldKind::LastDone {
            if self.editing == Some(index) {
                return base.child(
                    div()
                        .w(px(72.0))
                        .border_1()
                        .border_color(rgb(0x003C_59E7))
                        .child(self.draft.clone()),
                );
            }
            let price = self.price(index).unwrap_or(row.quote.prev_close);
            let color = if price > row.quote.prev_close {
                green()
            } else {
                red()
            };
            return base.text_color(color).child(format!("{price:.3}"));
        }
        let cell = base.child(row.text(field.kind));
        if field.tint {
            cell.text_color(tint)
        } else {
            cell
        }
    }
}

fn header() -> Div {
    let mut element = div()
        .flex()
        .flex_row()
        .h(px(HEADER_H))
        .border_b_1()
        .border_color(rgb(0x00E0_E0E0))
        .py_0p5()
        .px_2();
    for field in &FIELDS {
        element = element.child(
            div()
                .whitespace_nowrap()
                .truncate()
                .w(px(field.width))
                .px_1()
                .child(header_label(field.key)),
        );
    }
    element
}

fn button(label: impl Into<String>) -> Div {
    div()
        .h(px(28.0))
        .px_2()
        .border_1()
        .border_color(rgb(0x00C0_C0C0))
        .rounded_sm()
        .child(label.into())
}
