#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]

#[cfg(feature = "gpui-window")]
use std::ops::Range;
#[cfg(feature = "gpui-window")]
use std::sync::{Arc, Mutex};
#[cfg(feature = "gpui-window")]
use std::time::{Duration, Instant};

#[cfg(feature = "gpui-window")]
use access_log::{
    AccessLine, HEADERS, ROW_H, TICK_MS, VIEW_H, VIEW_W, col_w, generate, row_count, tick_visible,
    visible_rows,
};
#[cfg(feature = "gpui-window")]
use gpui::{
    App, Application, Bounds, Context, ScrollStrategy, SharedString, Timer,
    UniformListScrollHandle, Window, WindowBounds, WindowOptions, div, prelude::*, px, rgb, size,
    uniform_list,
};

#[cfg(feature = "gpui-window")]
struct AccessLog {
    book: Arc<Mutex<Vec<AccessLine>>>,
    scroll: UniformListScrollHandle,
    last_first: usize,
    last_rebuild: Duration,
}

#[cfg(feature = "gpui-window")]
impl AccessLog {
    fn tick(&mut self, cx: &mut Context<'_, Self>) {
        let vis = visible_rows();
        if let Ok(mut book) = self.book.lock() {
            tick_visible(&mut book, self.last_first, vis);
        }
        cx.notify();
    }
}

#[cfg(feature = "gpui-window")]
impl Render for AccessLog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let start = Instant::now();
        let book = self.book.clone();
        let n = book.lock().map(|rows| rows.len()).unwrap_or(0);
        let handle = self.scroll.clone();
        let handle_top = self.scroll.clone();
        let handle_tail = self.scroll.clone();
        let w = col_w();
        let hud = access_log::hud_line("GPUI", self.last_rebuild.as_nanos() as f64 / 1_000.0);
        let root = div()
            .flex()
            .flex_col()
            .gap_2()
            .p_3()
            .bg(rgb(0x00ff_ffff))
            .size_full()
            .child(div().text_xl().child(hud))
            .child(
                div()
                    .text_sm()
                    .child("rebuilds the element tree every 16 ms · watch seq/bytes"),
            )
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(
                        div()
                            .id("top")
                            .px_2()
                            .cursor_pointer()
                            .on_click(move |_, _, _| {
                                handle_top.scroll_to_item(0, ScrollStrategy::Top);
                            })
                            .child("Top"),
                    )
                    .child(
                        div()
                            .id("tail")
                            .px_2()
                            .cursor_pointer()
                            .on_click(move |_, _, _| {
                                handle_tail
                                    .scroll_to_item(n.saturating_sub(1), ScrollStrategy::Top);
                            })
                            .child("Tail"),
                    ),
            )
            .child(header_row(w))
            .child(
                uniform_list(
                    "access-log",
                    n,
                    cx.processor(move |this, range: Range<usize>, _, _| {
                        this.last_first = range.start;
                        let guard = book.lock().ok();
                        range
                            .map(|index| {
                                let line = guard
                                    .as_ref()
                                    .and_then(|rows| rows.get(index).copied())
                                    .unwrap_or_else(|| AccessLine::at(index));
                                tape_row(index, line, w)
                            })
                            .collect()
                    }),
                )
                .track_scroll(handle)
                .w(px(VIEW_W))
                .h(px(VIEW_H)),
            )
            .child(
                div()
                    .text_xs()
                    .child(format!("Held {n} lines · {}-row window", visible_rows())),
            );
        self.last_rebuild = start.elapsed();
        root
    }
}

#[cfg(feature = "gpui-window")]
fn header_row(w: f32) -> impl IntoElement {
    let mut row = div().flex().flex_row().h(px(20.0)).w(px(VIEW_W));
    for header in HEADERS {
        row = row.child(
            div()
                .w(px(w))
                .h(px(20.0))
                .child(SharedString::from(header.to_string())),
        );
    }
    row
}

#[cfg(feature = "gpui-window")]
fn tape_row(index: usize, line: AccessLine, w: f32) -> impl IntoElement {
    let mut row = div().flex().flex_row().h(px(ROW_H)).w(px(VIEW_W));
    for col in 0..HEADERS.len() {
        row = row.child(
            div()
                .w(px(w))
                .h(px(ROW_H))
                .child(SharedString::from(line.cell(index, col))),
        );
    }
    row
}

#[cfg(feature = "gpui-window")]
fn main() {
    let n = row_count();
    eprintln!("gpui live tape: allocating {n} lines");
    Application::new().run(move |cx: &mut App| {
        let book = Arc::new(Mutex::new(generate(n)));
        let bounds = Bounds::centered(None, size(px(1280.0), px(1080.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            move |_, cx| {
                cx.new(|cx| {
                    let this = AccessLog {
                        book: book.clone(),
                        scroll: UniformListScrollHandle::new(),
                        last_first: 0,
                        last_rebuild: Duration::ZERO,
                    };
                    cx.spawn(async move |this, cx| {
                        loop {
                            Timer::after(Duration::from_millis(TICK_MS)).await;
                            this.update(cx, |this: &mut AccessLog, cx| this.tick(cx))
                                .ok();
                        }
                    })
                    .detach();
                    this
                })
            },
        )
        .expect("example harness");
    });
}

#[cfg(not(feature = "gpui-window"))]
fn main() {}
