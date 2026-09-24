#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]
#![allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
#![allow(clippy::cast_sign_loss, clippy::doc_markdown)]
#![allow(clippy::print_stdout, clippy::too_many_lines)]

use sigil::app::DefaultTextMeasure;

use sigil::prelude::*;
use sigil::widgets::patterns;

use sigil_kittest::{
    InteractionMode, assert_png_matches_golden, golden_path, render_widget_to_png_stub,
    render_widget_to_png_with,
};

const DELTA_PCT: f32 = 0.5;

fn check(stem: &str, img: sigil_kittest::Rendered) {
    let Some(img) = img.expect("sigil-kittest") else {
        println!("no GPU adapter — skipping {stem}");
        return;
    };
    assert_png_matches_golden(&img, &golden_path!(stem), DELTA_PCT);
}

fn on_black(inner: impl IntoNode + 'static, w: f32, h: f32) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_column()
                .fixed_size(w, h)
                .padding(6.0)
                .align_items(AlignItems::Stretch),
        )
        .child(inner)
}

fn ten_row_list() -> Vec<String> {
    (0..10).map(|i| format!("item {i}")).collect()
}

fn row_label(label: &str) -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_row())
        .class("bg-surface text-on-surface")
        .child(text(label.to_owned()))
}

#[test]
fn virtual_list_default() {
    let img = render_widget_to_png_stub(
        |cx| {
            let w = virtual_list(ten_row_list(), |_, s: &String| row_label(s))
                .row_height(20.0)
                .viewport_height(200.0)
                .viewport_width(220.0);
            cx.add(on_black(w, 232.0, 212.0))
        },
        InteractionMode::Default,
        [232, 212],
        (120.0, 106.0),
    );
    check("virtual_list_default", img);
}

#[test]
fn virtual_list_focused() {
    let img = render_widget_to_png_stub(
        |cx| {
            let w = virtual_list(ten_row_list(), |_, s: &String| row_label(s))
                .row_height(20.0)
                .viewport_height(200.0)
                .viewport_width(220.0);
            cx.add(on_black(w, 232.0, 212.0))
        },
        InteractionMode::Focused,
        [232, 212],
        (120.0, 106.0),
    );
    check("virtual_list_focused", img);
}

#[test]
fn virtual_list_items_bind() {
    let img = render_widget_to_png_stub(
        |cx| {
            let items: Signal<Vec<String>> = signal(ten_row_list());
            let w = virtual_list(items, |_, s: &String| row_label(s))
                .row_height(20.0)
                .viewport_height(200.0)
                .viewport_width(220.0);
            cx.add(on_black(w, 232.0, 212.0))
        },
        InteractionMode::Default,
        [232, 212],
        (120.0, 106.0),
    );
    check("virtual_list_items_bind", img);
}

fn splitter_pane(class: &'static str) -> NodeBuilder {
    container()
        .class(class)
        .layout(LayoutStyle::flex_column().fill())
}

#[test]
fn splitter_horizontal_default() {
    let img = render_widget_to_png_stub(
        |cx| {
            let w = splitter(SplitterOrientation::Horizontal)
                .child(splitter_pane("bg-accent"), 120.0)
                .child(splitter_pane("bg-surface-hover"), 120.0)
                .layout(LayoutStyle::flex_row().fixed_size(246.0, 80.0));
            cx.add(on_black(w, 258.0, 92.0))
        },
        InteractionMode::Default,
        [258, 92],
        (168.0, 46.0),
    );
    check("splitter_horizontal_default", img);
}

#[test]
fn splitter_vertical_default() {
    let img = render_widget_to_png_stub(
        |cx| {
            let w = splitter(SplitterOrientation::Vertical)
                .child(splitter_pane("bg-accent"), 60.0)
                .child(splitter_pane("bg-surface-hover"), 60.0)
                .layout(LayoutStyle::flex_column().fixed_size(160.0, 126.0));
            cx.add(on_black(w, 172.0, 138.0))
        },
        InteractionMode::Default,
        [172, 138],
        (86.0, 70.0),
    );
    check("splitter_vertical_default", img);
}

#[test]
fn splitter_horizontal_with_min_max() {
    let img = render_widget_to_png_stub(
        |cx| {
            let w = splitter(SplitterOrientation::Horizontal)
                .child(splitter_pane("bg-accent"), 120.0)
                .child(splitter_pane("bg-surface-hover"), 120.0)
                .min_size(0, 60.0)
                .max_size(0, 200.0)
                .min_size(1, 60.0)
                .max_size(1, 200.0)
                .layout(LayoutStyle::flex_row().fixed_size(246.0, 80.0));
            cx.add(on_black(w, 258.0, 92.0))
        },
        InteractionMode::Default,
        [258, 92],
        (168.0, 46.0),
    );
    check("splitter_horizontal_with_min_max", img);
}

#[test]
fn text_edit_multiline() {
    let img = render_widget_to_png_stub(
        |cx| {
            let buffer = signal(String::from("first line\nsecond line\nthird line"));
            cx.add(text_edit(buffer).width(360.0).height(160.0))
        },
        InteractionMode::Default,
        [220, 88],
        (60.0, 18.0),
    );
    check("text_edit_multiline", img);
}

fn chip_in(label: &'static str, tone: Tone) -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_row())
        .child(chip(label, tone))
}

#[test]
fn chip_neutral() {
    let img = render_widget_to_png_stub(
        |cx| cx.add(chip_in("READY", Tone::Neutral)),
        InteractionMode::Default,
        [50, 35],
        (42.0, 29.4),
    );
    check("chip_neutral", img);
}

#[test]
fn chip_info() {
    let img = render_widget_to_png_stub(
        |cx| cx.add(chip_in("INFO", Tone::Info)),
        InteractionMode::Default,
        [50, 35],
        (42.0, 29.4),
    );
    check("chip_info", img);
}

#[test]
fn chip_success() {
    let img = render_widget_to_png_stub(
        |cx| cx.add(chip_in("OK", Tone::Success)),
        InteractionMode::Default,
        [50, 35],
        (42.0, 29.4),
    );
    check("chip_success", img);
}

#[test]
fn chip_warn() {
    let img = render_widget_to_png_stub(
        |cx| cx.add(chip_in("WARN", Tone::Warn)),
        InteractionMode::Default,
        [50, 35],
        (42.0, 29.4),
    );
    check("chip_warn", img);
}

#[test]
fn chip_error() {
    let img = render_widget_to_png_stub(
        |cx| cx.add(chip_in("ERROR", Tone::Error)),
        InteractionMode::Default,
        [50, 35],
        (42.0, 29.4),
    );
    check("chip_error", img);
}

#[test]
fn divider_horizontal_default() {
    let img = render_widget_to_png_stub(
        |cx| {
            cx.add(
                container()
                    .layout(LayoutStyle::flex_column().fixed_size(180.0, 60.0))
                    .child(
                        container()
                            .class("bg-accent")
                            .layout(LayoutStyle::flex_row().fill_width().height(20.0)),
                    )
                    .child(divider_horizontal())
                    .child(
                        container()
                            .class("bg-surface-hover")
                            .layout(LayoutStyle::flex_row().fill_width().height(20.0)),
                    ),
            )
        },
        InteractionMode::Default,
        [192, 72],
        (90.0, 30.0),
    );
    check("divider_horizontal_default", img);
}

#[test]
fn divider_vertical_default() {
    let img = render_widget_to_png_stub(
        |cx| {
            cx.add(
                container()
                    .layout(LayoutStyle::flex_row().fixed_size(120.0, 60.0))
                    .child(
                        container()
                            .class("bg-accent")
                            .layout(LayoutStyle::flex_column().fill_height().width(40.0)),
                    )
                    .child(divider_vertical())
                    .child(
                        container()
                            .class("bg-surface-hover")
                            .layout(LayoutStyle::flex_column().fill_height().width(40.0)),
                    ),
            )
        },
        InteractionMode::Default,
        [136, 72],
        (80.0, 32.0),
    );
    check("divider_vertical_default", img);
}

#[test]
fn kv_row_default() {
    let img = render_widget_to_png_stub(
        |cx| {
            let row = container()
                .layout(LayoutStyle::flex_column().fixed_size(220.0, 40.0))
                .class("bg-surface")
                .child(kv_row("Status", text("200 OK").class("font-mono")));
            cx.add(on_black(row, 232.0, 52.0))
        },
        InteractionMode::Default,
        [232, 52],
        (116.0, 26.0),
    );
    check("kv_row_default", img);
}

#[test]
fn status_bar_default() {
    let img = render_widget_to_png_stub(
        |cx| {
            let bar = container()
                .layout(LayoutStyle::flex_column().fixed_size(280.0, 48.0))
                .class("bg-surface")
                .child(status_bar([
                    chip("READY", Tone::Success),
                    chip("3 ITEMS", Tone::Neutral),
                ]));
            cx.add(on_black(bar, 292.0, 60.0))
        },
        InteractionMode::Default,
        [292, 60],
        (146.0, 30.0),
    );
    check("status_bar_default", img);
}

#[test]
fn number_field_default() {
    let img = render_widget_to_png_stub(
        |cx| {
            let value = signal(42i32);
            cx.add(number_field(value, 0..=100).text_field())
        },
        InteractionMode::Default,
        [192, 44],
        (40.0, 22.0),
    );
    check("number_field_default", img);
}

#[test]
fn number_field_focused() {
    let img = render_widget_to_png_stub(
        |cx| {
            let value = signal(42i32);
            cx.add(number_field(value, 0..=100).text_field())
        },
        InteractionMode::Focused,
        [192, 44],
        (40.0, 22.0),
    );
    check("number_field_focused", img);
}

#[test]
fn vector_path_shapes() {
    let img = render_widget_to_png_stub(
        |cx| {
            let accent = sigil::style::Rgba8 {
                r: 80,
                g: 130,
                b: 255,
                a: 255,
            };
            let warn = sigil::style::Rgba8 {
                r: 240,
                g: 160,
                b: 40,
                a: 255,
            };
            let row = container()
                .layout(
                    LayoutStyle::flex_row()
                        .fixed_size(150.0, 76.0)
                        .gap(12.0)
                        .padding(8.0),
                )
                .class("bg-surface")
                .child(
                    path(60, 60)
                        .move_to(30.0, 4.0)
                        .line_to(56.0, 56.0)
                        .line_to(4.0, 56.0)
                        .close()
                        .fill(accent),
                )
                .child(
                    path(60, 60)
                        .move_to(6.0, 54.0)
                        .cubic_to(22.0, 6.0, 38.0, 6.0, 54.0, 54.0)
                        .stroke(warn, 5.0),
                );
            cx.add(on_black(row, 162.0, 88.0))
        },
        InteractionMode::Default,
        [162, 88],
        (1.0, 1.0),
    );
    check("vector_path_shapes", img);
}

#[test]
fn vector_gradient_fill() {
    fn square(a: sigil::style::Rgba8, b: sigil::style::Rgba8, radial: bool) -> PathNode {
        let base = path(60, 60)
            .move_to(0.0, 0.0)
            .line_to(60.0, 0.0)
            .line_to(60.0, 60.0)
            .line_to(0.0, 60.0)
            .close();
        if radial {
            base.fill_radial((30.0, 30.0), 30.0, vec![(0.0, a), (1.0, b)])
        } else {
            base.fill_linear((0.0, 0.0), (60.0, 60.0), vec![(0.0, a), (1.0, b)])
        }
    }
    let img = render_widget_to_png_stub(
        |cx| {
            let gold = sigil::style::Rgba8 {
                r: 255,
                g: 210,
                b: 60,
                a: 255,
            };
            let rose = sigil::style::Rgba8 {
                r: 220,
                g: 40,
                b: 120,
                a: 255,
            };
            let row = container()
                .layout(
                    LayoutStyle::flex_row()
                        .fixed_size(150.0, 76.0)
                        .gap(12.0)
                        .padding(8.0),
                )
                .class("bg-surface")
                .child(square(gold, rose, true))
                .child(square(gold, rose, false));
            cx.add(on_black(row, 162.0, 88.0))
        },
        InteractionMode::Default,
        [162, 88],
        (1.0, 1.0),
    );
    check("vector_gradient_fill", img);
}

#[test]
fn emoji_text_line() {
    let img = render_widget_to_png_with(
        |cx| {
            let panel = container()
                .layout(
                    LayoutStyle::flex_column()
                        .fixed_size(300.0, 60.0)
                        .padding(16.0),
                )
                .class("bg-surface")
                .child(text("OK \u{2705} go \u{1F680}").class("text-on-surface"));
            cx.add(on_black(panel, 312.0, 72.0))
        },
        InteractionMode::Default,
        [312, 72],
        (1.0, 1.0),
        Box::new(DefaultTextMeasure::new()),
    );
    check("emoji_text_line", img);
}

#[test]
fn hint_overlay_default() {
    let img = render_widget_to_png_with(
        |cx| {
            let overlay = patterns::hint_overlay(&[
                ("Ctrl+S", "Save"),
                ("Ctrl+Z", "Undo"),
                ("?", "Toggle this overlay"),
            ]);
            let surface = container()
                .layout(LayoutStyle::flex_column().fixed_size(220.0, 84.0))
                .class("bg-surface")
                .child(overlay);
            cx.add(on_black(surface, 232.0, 96.0))
        },
        InteractionMode::Default,
        [232, 96],
        (1.0, 1.0),
        Box::new(DefaultTextMeasure::new()),
    );
    check("hint_overlay_default", img);
}

#[test]
fn pattern_app_shell() {
    let img = render_widget_to_png_stub(
        |cx| {
            cx.add(
                container()
                    .layout(LayoutStyle::flex_column().fixed_size(360.0, 200.0))
                    .child(patterns::app_shell(
                        container()
                            .class("bg-accent")
                            .layout(LayoutStyle::flex_row().fill().padding(8.0))
                            .child(text("Top bar")),
                        container()
                            .class("bg-surface-hover")
                            .layout(LayoutStyle::flex_column().fill().padding(8.0))
                            .child(text("Work")),
                        status_bar([chip("READY", Tone::Success)]),
                        None::<NodeBuilder>,
                    )),
            )
        },
        InteractionMode::Default,
        [376, 216],
        (1.0, 1.0),
    );
    check("pattern_app_shell", img);
}

#[test]
fn pattern_field() {
    let img = render_widget_to_png_stub(
        |cx| {
            let buffer = signal(String::from("you@host"));
            let field = container()
                .layout(
                    LayoutStyle::flex_column()
                        .fixed_size(220.0, 70.0)
                        .padding(8.0),
                )
                .class("bg-surface")
                .child(patterns::field("Username", text_field(buffer)));
            cx.add(on_black(field, 232.0, 82.0))
        },
        InteractionMode::Default,
        [232, 82],
        (116.0, 41.0),
    );
    check("pattern_field", img);
}

#[test]
fn pattern_kv_grid() {
    let img = render_widget_to_png_with(
        |cx| {
            let grid = container()
                .layout(
                    LayoutStyle::flex_column()
                        .fixed_size(280.0, 96.0)
                        .padding(8.0),
                )
                .class("bg-surface")
                .child(patterns::kv_grid(&[
                    ("Content-Type", "application/json"),
                    ("Status", "200 OK"),
                    ("Length", "1024"),
                ]));
            cx.add(on_black(grid, 292.0, 108.0))
        },
        InteractionMode::Default,
        [292, 108],
        (146.0, 54.0),
        Box::new(DefaultTextMeasure::new()),
    );
    check("pattern_kv_grid", img);
}
