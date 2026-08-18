//! Committed-PNG visual goldens for widgets that shipped after the
//! original `visual_goldens.rs` 13-widget catalog: `virtual_list`,
//! `splitter`, multi-line `text_edit`, `chip`, divider primitives,
//! `kv_row`, `status_bar`, `number_field`, `hint_overlay`, and the
//! `patterns::` composition shells (`app_shell`, `field`, `kv_grid`).
//!
//! Pattern mirrors `visual_goldens.rs` — each test builds a widget,
//! renders through the wgpu offscreen path, and asserts byte-equivalence
//! (within `DELTA_PCT`) against a committed PNG. Skips with `None` when
//! no GPU adapter is available. Set `SIGIL_REGEN_GOLDENS=1` to re-bake.

#![allow(missing_docs)]
#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]
#![allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
#![allow(clippy::cast_sign_loss, clippy::doc_markdown)]
#![allow(clippy::print_stdout, clippy::too_many_lines)]

use arboard as _;
use criterion as _;
use sigil_actions as _;
use sigil_animation as _;
use sigil_animation_runtime as _;
use sigil_app::DefaultTextMeasure;
use sigil_bidi as _;
use sigil_events as _;
use sigil_hal as _;
use sigil_kernel as _;
use sigil_layout as _;
use sigil_profile as _;
use sigil_reactive as _;
use sigil_style as _;
use sigil_tasks as _;

use sigil::prelude::*;
use sigil::widgets::patterns;

mod golden_support;

use golden_support::{
    InteractionMode, assert_png_matches_golden, golden_path, render_widget_to_png,
    render_widget_to_png_with,
};

const DELTA_PCT: f32 = 0.5;

fn check(stem: &str, img: Option<image::RgbaImage>) {
    let Some(img) = img else {
        println!("no GPU adapter — skipping {stem}");
        return;
    };
    assert_png_matches_golden(&img, &golden_path(stem), DELTA_PCT);
}

/// Wrap a widget in a frame-filling, black-background container with a
/// few px of padding. Canonical golden framing: the padding guarantees
/// the CORNER pixel is the (black) clear colour — the quality gate
/// measures coverage as "differs from the corner pixel", so a widget
/// that bleeds to the frame edge makes the corner part of the widget and
/// the coverage metric collapses. A small black border + a widget sized
/// to fill the rest yields ≥70 % coverage of bright themed pixels over
/// black, which is what "proper framing" means for the gate.
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

// ---- VIRTUAL_LIST ------------------------------------------------------------
//
// Materialises a ten-row list of static text labels inside a 240×220
// canvas. Row height + viewport height are dialed in so the full ten
// rows fit without scrolling — this keeps the rasterised pixels
// dominated by row content, not scroll bar / clip artifacts.

fn ten_row_list() -> Vec<String> {
    (0..10).map(|i| format!("item {i}")).collect()
}

/// Render one virtual_list row: a flex_row container holding the label
/// text. The `virtual_list` callback returns `NodeBuilder` (not the
/// `TextBuilder` `text()` produces), so wrap the label in a container.
fn row_label(label: &str) -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_row())
        .class("bg-surface text-on-surface")
        .child(text(label.to_owned()))
}

#[test]
fn virtual_list_default() {
    let img = render_widget_to_png(
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
    let img = render_widget_to_png(
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
    // Exercise the `items_bind` reactive path. Build with the externally
    // owned signal so the visible-window subtree wires through
    // `children_reactive` — the rendered PNG should match the static
    // form pixel-for-pixel since the initial Vec is identical.
    let img = render_widget_to_png(
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

// ---- SPLITTER ----------------------------------------------------------------
//
// Two coloured panes flank a 6 px drag handle. Horizontal lays them
// left-to-right; vertical stacks them top-to-bottom. Min/max variant
// proves the constraint surface composes — the rendered split is still
// at the initial size, so the golden only differs from default if the
// min/max landed (they do — initial is clamped if outside range).

fn splitter_pane(class: &'static str) -> NodeBuilder {
    container()
        .class(class)
        .layout(LayoutStyle::flex_column().fill())
}

#[test]
fn splitter_horizontal_default() {
    let img = render_widget_to_png(
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
    let img = render_widget_to_png(
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
    // Same shape as `splitter_horizontal_default` but applies per-pane
    // min/max constraints. Initial sizes are already inside the range so
    // the rendered split should be unchanged — this proves the
    // constraint API composes without breaking the layout.
    let img = render_widget_to_png(
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

// ---- TEXT_EDIT MULTI-LINE ----------------------------------------------------
//
// Three-line buffer demonstrating B4 (`children_reactive` over rope
// lines). The buffer is seeded with two embedded newlines so the
// rendered widget shows three rows of text stacked vertically.

#[test]
fn text_edit_multiline() {
    let img = render_widget_to_png(
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

// ---- CHIP --------------------------------------------------------------------
//
// Five tones map to five PNGs. Each golden captures the chip rendered
// at native size so the per-tone palette difference is visible to a
// pixel-diff and a vision review.

fn chip_in(label: &'static str, tone: Tone) -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_row())
        .child(chip(label, tone))
}

#[test]
fn chip_neutral() {
    let img = render_widget_to_png(
        |cx| cx.add(chip_in("READY", Tone::Neutral)),
        InteractionMode::Default,
        [50, 35],
        (42.0, 29.4),
    );
    check("chip_neutral", img);
}

#[test]
fn chip_info() {
    let img = render_widget_to_png(
        |cx| cx.add(chip_in("INFO", Tone::Info)),
        InteractionMode::Default,
        [50, 35],
        (42.0, 29.4),
    );
    check("chip_info", img);
}

#[test]
fn chip_success() {
    let img = render_widget_to_png(
        |cx| cx.add(chip_in("OK", Tone::Success)),
        InteractionMode::Default,
        [50, 35],
        (42.0, 29.4),
    );
    check("chip_success", img);
}

#[test]
fn chip_warn() {
    let img = render_widget_to_png(
        |cx| cx.add(chip_in("WARN", Tone::Warn)),
        InteractionMode::Default,
        [50, 35],
        (42.0, 29.4),
    );
    check("chip_warn", img);
}

#[test]
fn chip_error() {
    let img = render_widget_to_png(
        |cx| cx.add(chip_in("ERROR", Tone::Error)),
        InteractionMode::Default,
        [50, 35],
        (42.0, 29.4),
    );
    check("chip_error", img);
}

// ---- DIVIDER -----------------------------------------------------------------
//
// 1 px hairlines sitting inside a sized parent. Horizontal fills the
// inline axis; vertical fills the block axis. Tucked between two
// coloured siblings so the rule itself is visually distinguishable
// from the background.

#[test]
fn divider_horizontal_default() {
    let img = render_widget_to_png(
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
    let img = render_widget_to_png(
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

// ---- KV_ROW ------------------------------------------------------------------
//
// One row, label on the left, monospace value on the right. Sized
// parent so the `fill_width` row has somewhere to expand into.

#[test]
fn kv_row_default() {
    let img = render_widget_to_png(
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

// ---- STATUS_BAR --------------------------------------------------------------
//
// Two chips inside a status-bar row. Sized parent so the `fill_width`
// surface has a known width.

#[test]
fn status_bar_default() {
    let img = render_widget_to_png(
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

// ---- NUMBER_FIELD ------------------------------------------------------------
//
// Wraps the existing text_field with parse + clamp; the rendered
// surface IS the text_field, so the golden is dimensionally identical
// to `text_field_default.png` but with numeric content.

#[test]
fn number_field_default() {
    let img = render_widget_to_png(
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
    let img = render_widget_to_png(
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

// ---- HINT_OVERLAY ------------------------------------------------------------
//
// Three-row keybinding cheat sheet stacked vertically.

#[test]
fn vector_path_shapes() {
    // Arbitrary Bézier fill + stroke through the vector-path primitive:
    // a filled triangle and a stroked quadratic arc, rasterized (tiny-skia)
    // and painted via the image node. Proves the path pipeline end-to-end.
    let img = render_widget_to_png(
        |cx| {
            let accent = sigil_style::Rgba8 {
                r: 80,
                g: 130,
                b: 255,
                a: 255,
            };
            let warn = sigil_style::Rgba8 {
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
    // Radial + linear gradient fills on vector paths (tiny-skia shaders),
    // rendered through the image node. Closes the radial-gradient gap.
    fn square(a: sigil_style::Rgba8, b: sigil_style::Rgba8, radial: bool) -> PathNode {
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
    let img = render_widget_to_png(
        |cx| {
            let gold = sigil_style::Rgba8 {
                r: 255,
                g: 210,
                b: 60,
                a: 255,
            };
            let rose = sigil_style::Rgba8 {
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
    // Color emoji must paint in full color inline with the text, not as
    // monochrome silhouettes or .notdef tofu. Real measure so the bundled
    // Noto Color Emoji fallback and the glyph positions match the render.
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
            // Canonical framing (mirrors `kv_row_default`): mount the overlay on
            // a fixed-size `bg-surface` box that fills the inner crop, then wrap
            // in the `on_black` border. The themed surface fills ~83 % of the
            // 232×96 frame, clearing the ≥70 %-coverage gate. The bare overlay is
            // content-sized and left ~57 % of the frame black (42 % coverage).
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
        // Two-column chord/description rows: box widths are driven by real
        // glyph advances, so the deterministic 40px StubMeasure would place
        // the columns on top of each other. Use the production measure.
        Box::new(DefaultTextMeasure::new()),
    );
    check("hint_overlay_default", img);
}

// ---- PATTERNS::APP_SHELL -----------------------------------------------------
//
// Full four-section layout: top bar / work area / status bar, no modal
// overlay. The shell stretches to its `fill()` constraint so a sized
// outer wrapper is required to bound it.

#[test]
fn pattern_app_shell() {
    let img = render_widget_to_png(
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

// ---- PATTERNS::FIELD ---------------------------------------------------------
//
// Label-above-control vertical pair: label text + text_field.

#[test]
fn pattern_field() {
    let img = render_widget_to_png(
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

// ---- PATTERNS::KV_GRID -------------------------------------------------------
//
// Three-row key/value grid demonstrating the multi-row composition.

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
        // Key/value columns are sized from real glyph advances; the 40px
        // StubMeasure would overlap them. Use the production measure.
        Box::new(DefaultTextMeasure::new()),
    );
    check("pattern_kv_grid", img);
}
