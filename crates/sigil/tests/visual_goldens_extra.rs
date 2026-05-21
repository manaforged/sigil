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

use actions as _;
use animation as _;
use animation_runtime as _;
use app as _;
use arboard as _;
use criterion as _;
use events as _;
use hal as _;
use kernel as _;
use layout as _;
use reactive as _;
use style as _;
use tasks as _;

use sigil::prelude::*;
use sigil::widgets::patterns;

mod golden_support;

use golden_support::{
    InteractionMode, assert_png_matches_golden, golden_path, render_widget_to_png,
};

const DELTA_PCT: f32 = 0.5;

fn check(stem: &str, img: Option<image::RgbaImage>) {
    let Some(img) = img else {
        println!("no GPU adapter — skipping {stem}");
        return;
    };
    assert_png_matches_golden(&img, &golden_path(stem), DELTA_PCT);
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
        .child(text(label.to_owned()))
}

#[test]
fn virtual_list_default() {
    let img = render_widget_to_png(
        |cx| {
            cx.add(
                virtual_list(ten_row_list(), |_, s: &String| row_label(s))
                    .row_height(20.0)
                    .viewport_height(200.0)
                    .viewport_width(220.0),
            )
        },
        InteractionMode::Default,
        [240, 220],
        (120.0, 100.0),
    );
    check("virtual_list_default", img);
}

#[test]
fn virtual_list_focused() {
    let img = render_widget_to_png(
        |cx| {
            cx.add(
                virtual_list(ten_row_list(), |_, s: &String| row_label(s))
                    .row_height(20.0)
                    .viewport_height(200.0)
                    .viewport_width(220.0),
            )
        },
        InteractionMode::Focused,
        [240, 220],
        (120.0, 100.0),
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
            cx.add(
                virtual_list(Vec::<String>::new(), |_, s: &String| row_label(s))
                    .items_bind(items)
                    .row_height(20.0)
                    .viewport_height(200.0)
                    .viewport_width(220.0),
            )
        },
        InteractionMode::Default,
        [240, 220],
        (120.0, 100.0),
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
            cx.add(
                splitter(SplitterOrientation::Horizontal)
                    .child(splitter_pane("bg-accent"), 120.0)
                    .child(splitter_pane("bg-surface-hover"), 120.0)
                    .layout(LayoutStyle::flex_row().fixed_size(246.0, 80.0)),
            )
        },
        InteractionMode::Default,
        [256, 96],
        (123.0, 40.0),
    );
    check("splitter_horizontal_default", img);
}

#[test]
fn splitter_vertical_default() {
    let img = render_widget_to_png(
        |cx| {
            cx.add(
                splitter(SplitterOrientation::Vertical)
                    .child(splitter_pane("bg-accent"), 60.0)
                    .child(splitter_pane("bg-surface-hover"), 60.0)
                    .layout(LayoutStyle::flex_column().fixed_size(160.0, 126.0)),
            )
        },
        InteractionMode::Default,
        [176, 144],
        (80.0, 63.0),
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
            cx.add(
                splitter(SplitterOrientation::Horizontal)
                    .child(splitter_pane("bg-accent"), 120.0)
                    .child(splitter_pane("bg-surface-hover"), 120.0)
                    .min_size(0, 60.0)
                    .max_size(0, 200.0)
                    .min_size(1, 60.0)
                    .max_size(1, 200.0)
                    .layout(LayoutStyle::flex_row().fixed_size(246.0, 80.0)),
            )
        },
        InteractionMode::Default,
        [256, 96],
        (123.0, 40.0),
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
            cx.add(text_edit(buffer))
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

fn chip_in(label: &'static str, tone: ChipTone) -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_row().padding(8.0))
        .child(chip(label, tone))
}

#[test]
fn chip_neutral() {
    let img = render_widget_to_png(
        |cx| cx.add(chip_in("READY", ChipTone::Neutral)),
        InteractionMode::Default,
        [120, 40],
        (1.0, 1.0),
    );
    check("chip_neutral", img);
}

#[test]
fn chip_info() {
    let img = render_widget_to_png(
        |cx| cx.add(chip_in("INFO", ChipTone::Info)),
        InteractionMode::Default,
        [120, 40],
        (1.0, 1.0),
    );
    check("chip_info", img);
}

#[test]
fn chip_success() {
    let img = render_widget_to_png(
        |cx| cx.add(chip_in("OK", ChipTone::Success)),
        InteractionMode::Default,
        [120, 40],
        (1.0, 1.0),
    );
    check("chip_success", img);
}

#[test]
fn chip_warn() {
    let img = render_widget_to_png(
        |cx| cx.add(chip_in("WARN", ChipTone::Warn)),
        InteractionMode::Default,
        [120, 40],
        (1.0, 1.0),
    );
    check("chip_warn", img);
}

#[test]
fn chip_error() {
    let img = render_widget_to_png(
        |cx| cx.add(chip_in("ERROR", ChipTone::Error)),
        InteractionMode::Default,
        [120, 40],
        (1.0, 1.0),
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
        (60.0, 30.0),
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
            cx.add(
                container()
                    .layout(
                        LayoutStyle::flex_column()
                            .fixed_size(220.0, 40.0)
                            .padding(6.0),
                    )
                    .child(kv_row("Status", text("200 OK").class("font-mono"))),
            )
        },
        InteractionMode::Default,
        [240, 48],
        (1.0, 1.0),
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
            cx.add(
                container()
                    .layout(LayoutStyle::flex_column().fixed_size(280.0, 48.0))
                    .child(status_bar([
                        chip("READY", ChipTone::Success),
                        chip("3 ITEMS", ChipTone::Neutral),
                    ])),
            )
        },
        InteractionMode::Default,
        [296, 56],
        (1.0, 1.0),
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
fn hint_overlay_default() {
    let img = render_widget_to_png(
        |cx| {
            cx.add(patterns::hint_overlay(&[
                ("Ctrl+S", "Save"),
                ("Ctrl+Z", "Undo"),
                ("?", "Toggle this overlay"),
            ]))
        },
        InteractionMode::Default,
        [260, 120],
        (1.0, 1.0),
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
                        status_bar([chip("READY", ChipTone::Success)]),
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
            cx.add(
                container()
                    .layout(
                        LayoutStyle::flex_column()
                            .fixed_size(220.0, 70.0)
                            .padding(6.0),
                    )
                    .child(patterns::field("Username", text_field(buffer))),
            )
        },
        InteractionMode::Default,
        [240, 88],
        (1.0, 1.0),
    );
    check("pattern_field", img);
}

// ---- PATTERNS::KV_GRID -------------------------------------------------------
//
// Three-row key/value grid demonstrating the multi-row composition.

#[test]
fn pattern_kv_grid() {
    let img = render_widget_to_png(
        |cx| {
            cx.add(
                container()
                    .layout(
                        LayoutStyle::flex_column()
                            .fixed_size(280.0, 96.0)
                            .padding(6.0),
                    )
                    .child(patterns::kv_grid(&[
                        ("Content-Type", "application/json"),
                        ("Status", "200 OK"),
                        ("Length", "1024"),
                    ])),
            )
        },
        InteractionMode::Default,
        [296, 112],
        (1.0, 1.0),
    );
    check("pattern_kv_grid", img);
}
