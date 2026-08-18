//! Sigil live tape: 1M lines, visible band ticks at 60 Hz, rows kept.

use std::rc::Rc;
use std::time::{Duration, Instant};

use access_log::{
    AccessLine, HEADERS, ROW_H, TICK_MS, VIEW_H, VIEW_W, col_w, first_at, generate, row_count,
    tick_visible, visible_rows, window_at,
};
use sigil::prelude::*;
use sigil::widgets::virtual_list_bind;

fn main() -> sigil::AppResult<()> {
    let n = row_count();
    eprintln!("sigil live tape: allocating {n} lines");
    sigil::launch(("Sigil — live tape", 1280, 1080), build_root)
}

fn build_root(cx: &mut Cx) -> NodeId {
    build(cx, row_count())
}

fn build(cx: &mut Cx, n: usize) -> NodeId {
    let book = Rc::new(generate(n));
    let held = book.len();
    let rows = signal(book);
    let scroll = signal(0.0_f32);
    let hud = signal(String::from("SIGIL  — µs"));
    let max_y = (held as f32).mul_add(ROW_H, -VIEW_H).max(0.0);
    let vis = visible_rows();
    cx.interval(Duration::from_millis(TICK_MS), {
        let rows = rows;
        let scroll = scroll;
        let hud = hud;
        move || {
            let y = scroll.peek();
            let (first, end) = window_at(y, held);
            let start = Instant::now();
            rows.update(|book| {
                tick_visible(
                    Rc::make_mut(book).as_mut_slice(),
                    first,
                    end.saturating_sub(first),
                );
            });
            let us = start.elapsed().as_nanos() as f64 / 1_000.0;
            hud.set(access_log::hud_line("SIGIL", us));
        }
    });
    let status = derived(move || {
        let y = scroll.get();
        let in_view = first_at(y);
        let (first, end) = window_at(y, held);
        format!(
            "Held {held} lines · view {in_view}..{} · mounted {first}..{end} · {vis} in view",
            in_view.saturating_add(vis).min(held)
        )
    });
    cx.add(
        container()
            .role(Role::Main)
            .label("Live tape")
            .layout(
                LayoutStyle::flex_column()
                    .fixed_size(1280.0, 1080.0)
                    .gap(6.0)
                    .padding(12.0),
            )
            .class(classes!("bg-surface"))
            .child(text(hud).class(classes!("text-xl font-mono")))
            .child(
                text("retained tree · visible band ticks every 16 ms · watch seq/bytes")
                    .class(classes!("text-sm")),
            )
            .child(
                container()
                    .layout(LayoutStyle::flex_row().gap(8.0).height(28.0))
                    .child(button("Top").activate({
                        let scroll = scroll;
                        move || scroll.set(0.0)
                    }))
                    .child(button("Tail").primary().activate({
                        let scroll = scroll;
                        move || scroll.set(max_y)
                    })),
            )
            .child(header_row())
            .child(
                container()
                    .layout(LayoutStyle::block().fixed_size(VIEW_W, VIEW_H))
                    .class(classes!("overflow-hidden"))
                    .child(
                        virtual_list_bind(rows, move |index, line: &AccessLine| {
                            tape_row(index, line, rows)
                        })
                        .row_height(ROW_H)
                        .viewport_height(VIEW_H)
                        .viewport_width(VIEW_W)
                        .scroll_bind(scroll)
                        .keyboard(false),
                    ),
            )
            .child(text(status).class(classes!("text-xs"))),
    )
}

fn header_row() -> NodeBuilder {
    let mut row = container().layout(LayoutStyle::flex_row().fixed_size(VIEW_W, 20.0));
    let w = col_w();
    for header in HEADERS {
        row = row.child(
            container()
                .layout(LayoutStyle::block().fixed_size(w, 20.0))
                .child(text(header).nowrap().class(classes!("text-xs"))),
        );
    }
    row
}

fn tape_row(index: usize, line: &AccessLine, book: Signal<Rc<Vec<AccessLine>>>) -> NodeBuilder {
    let w = col_w();
    let mut row = container().layout(LayoutStyle::flex_row().fixed_size(VIEW_W, ROW_H));
    for col in 0..HEADERS.len() {
        let value = if col == 4 || col == 5 {
            TextContent::compute(move || {
                book.get()
                    .get(index)
                    .map_or_else(String::new, |line| line.cell(index, col))
            })
        } else {
            TextContent::Static(line.cell(index, col))
        };
        row = row.child(
            container()
                .layout(LayoutStyle::block().fixed_size(w, ROW_H))
                .child(text(value).nowrap().class(classes!("text-xs"))),
        );
    }
    row
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use sigil::testing::with_cx;

    #[test]
    fn tree_builds_without_panic() {
        with_cx("access-log-sigil", |cx| {
            let root = build(cx, 200);
            assert!(cx.child_count(root) > 0);
        });
    }
}
