#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
//! Wave-2 dogfood widget goldens — icon, dot, radio_group, anchored menu,
//! validated_field.
//!
//! Bake with `SIGIL_REGEN_GOLDENS=1 cargo test -p sigil-ui --test
//! visual_goldens_wave2`. Skips when no GPU adapter is present.

#![allow(missing_docs)]
#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]
#![allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
#![allow(clippy::cast_sign_loss, clippy::doc_markdown)]
#![allow(clippy::print_stdout, clippy::too_many_lines)]

use sigil::prelude::*;
use sigil_style::Size as ControlSize;

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

// ---- ICON --------------------------------------------------------------------
//
// Test glyphs are deliberately ASCII (`i`, `+`, `x`) so the bundled
// Fira Sans Regular fixture font has them — Fira Sans does not
// include the U+2600 Miscellaneous Symbols block, so U+2699 (⚙) and
// similar icon-font glyphs render as the "?" missing-glyph fallback
// in the goldens. The icon widget itself stays font-agnostic — these
// tests validate the rendering path, not the consumer's icon font.

#[test]
fn icon_default() {
    let img = render_widget_to_png(
        |cx| {
            cx.add(
                container()
                    .class("bg-surface")
                    .layout(LayoutStyle::flex_row().fill())
                    .child(icon("i")),
            )
        },
        InteractionMode::Default,
        [32, 32],
        (16.0, 16.0),
    );
    check("icon_default", img);
}

#[test]
fn icon_primary_tone() {
    let img = render_widget_to_png(
        |cx| {
            cx.add(
                container()
                    .class("bg-surface")
                    .layout(LayoutStyle::flex_row().fill())
                    .child(icon("+").tone(Tone::Primary)),
            )
        },
        InteractionMode::Default,
        [32, 32],
        (16.0, 16.0),
    );
    check("icon_primary_tone", img);
}

#[test]
fn icon_error_tone() {
    let img = render_widget_to_png(
        |cx| {
            cx.add(
                container()
                    .class("bg-surface")
                    .layout(LayoutStyle::flex_row().fill())
                    .child(icon("x").tone(Tone::Error)),
            )
        },
        InteractionMode::Default,
        [32, 32],
        (16.0, 16.0),
    );
    check("icon_error_tone", img);
}

// ---- DOT ---------------------------------------------------------------------

#[test]
fn dot_success() {
    // Wrap in a surface container so the frame background is the theme surface
    // (near-white) rather than the renderer clear-color (black). The 8 px dot
    // alone in a 32×32 frame covers ~5% of pixels — below the hook's coverage
    // gate. The surface wrapper fills the frame so coverage is 100% of a
    // light-vs-black comparison, and the dot still contrasts against the surface.
    let img = render_widget_to_png(
        |cx| {
            cx.add(
                container()
                    .class("bg-surface")
                    .layout(LayoutStyle::flex_row().fill())
                    .child(dot(Tone::Success)),
            )
        },
        InteractionMode::Default,
        [32, 32],
        (16.0, 16.0),
    );
    check("dot_success", img);
}

#[test]
fn dot_warn_default_size() {
    let img = render_widget_to_png(
        |cx| {
            cx.add(
                container()
                    .class("bg-surface")
                    .layout(LayoutStyle::flex_row().fill())
                    .child(dot(Tone::Warn).size(ControlSize::Default)),
            )
        },
        InteractionMode::Default,
        [32, 32],
        (16.0, 16.0),
    );
    check("dot_warn_default_size", img);
}

#[test]
fn dot_error_wide() {
    let img = render_widget_to_png(
        |cx| {
            cx.add(
                container()
                    .class("bg-surface")
                    .layout(LayoutStyle::flex_row().fill())
                    .child(dot(Tone::Error).size(ControlSize::Wide)),
            )
        },
        InteractionMode::Default,
        [32, 32],
        (16.0, 16.0),
    );
    check("dot_error_wide", img);
}

// ---- RADIO_GROUP -------------------------------------------------------------

#[test]
fn radio_group_three_options_first_selected() {
    let img = render_widget_to_png(
        |cx| {
            let selected = signal(String::from("light"));
            cx.add(
                container()
                    .class("bg-surface")
                    .layout(LayoutStyle::flex_row().fill())
                    .child(
                        radio_group(selected)
                            .option("light", "Light")
                            .option("dark", "Dark")
                            .option("auto", "Match system"),
                    ),
            )
        },
        InteractionMode::Default,
        [220, 120],
        (10.0, 14.0),
    );
    check("radio_group_three_options_first_selected", img);
}

#[test]
fn radio_group_middle_selected() {
    let img = render_widget_to_png(
        |cx| {
            let selected = signal(String::from("dark"));
            cx.add(
                container()
                    .class("bg-surface")
                    .layout(LayoutStyle::flex_row().fill())
                    .child(
                        radio_group(selected)
                            .option("light", "Light")
                            .option("dark", "Dark")
                            .option("auto", "Match system"),
                    ),
            )
        },
        InteractionMode::Default,
        [220, 120],
        (10.0, 42.0),
    );
    check("radio_group_middle_selected", img);
}

// ---- VALIDATED_FIELD ---------------------------------------------------------

#[test]
fn validated_field_no_error() {
    let img = render_widget_to_png(
        |cx| {
            let error: Signal<Option<String>> = signal(None);
            cx.add(
                container()
                    .class("bg-surface")
                    .layout(LayoutStyle::flex_row().fill())
                    .child(patterns::validated_field(
                        "Email",
                        text_field(signal(String::new())),
                        error,
                    )),
            )
        },
        InteractionMode::Default,
        [200, 52],
        (10.0, 10.0),
    );
    check("validated_field_no_error", img);
}

#[test]
fn validated_field_with_error() {
    let img = render_widget_to_png(
        |cx| {
            let error: Signal<Option<String>> = signal(Some(String::from("Email must contain @")));
            cx.add(
                container()
                    .class("bg-surface")
                    .layout(LayoutStyle::flex_row().fill())
                    .child(patterns::validated_field(
                        "Email",
                        text_field(signal(String::new())),
                        error,
                    )),
            )
        },
        InteractionMode::Default,
        [200, 80],
        (10.0, 10.0),
    );
    check("validated_field_with_error", img);
}
