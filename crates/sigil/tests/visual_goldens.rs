//! Committed-PNG visual goldens for sigil widgets.
//!
//! Each (widget, state) combo renders through the wgpu offscreen path and
//! asserts byte-equivalent (within a 0.5% pixel-delta budget) against a
//! committed PNG at `crates/sigil/tests/goldens/`. Run with
//! `SIGIL_REGEN_GOLDENS=1` to re-bake — and always eyeball every regenerated
//! file in an image viewer before committing.
//!
//! Skips cleanly when no wgpu adapter is available (CI without GPU).

#![allow(missing_docs)]
#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]
#![allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
#![allow(clippy::cast_sign_loss, clippy::doc_markdown)]
#![allow(clippy::print_stdout, clippy::too_many_lines)]

use criterion as _;
use sigil_actions as _;
use sigil_app as _;
use sigil_events as _;
use sigil_hal as _;
use sigil_kernel as _;
use sigil_layout as _;
use sigil_reactive as _;
use sigil_style as _;

use sigil::prelude::*;

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

// ---- BUTTON ------------------------------------------------------------------

#[test]
fn button_default() {
    let img = render_widget_to_png(
        |cx| cx.add(button("Save")),
        InteractionMode::Default,
        [160, 56],
        (74.0, 22.0),
    );
    check("button_default", img);
}

#[test]
fn button_hover() {
    let img = render_widget_to_png(
        |cx| cx.add(button("Save")),
        InteractionMode::Hover,
        [160, 56],
        (74.0, 22.0),
    );
    check("button_hover", img);
}

#[test]
fn button_pressed() {
    let img = render_widget_to_png(
        |cx| cx.add(button("Save")),
        InteractionMode::Pressed,
        [160, 56],
        (74.0, 22.0),
    );
    check("button_pressed", img);
}

#[test]
fn button_focused() {
    let img = render_widget_to_png(
        |cx| cx.add(button("Save")),
        InteractionMode::Focused,
        [160, 56],
        (74.0, 22.0),
    );
    check("button_focused", img);
}

#[test]
fn button_disabled() {
    let img = render_widget_to_png(
        |cx| cx.add(button("Save").enabled(false)),
        InteractionMode::Disabled,
        [160, 56],
        (74.0, 22.0),
    );
    check("button_disabled", img);
}

// ---- CHECKBOX ----------------------------------------------------------------

#[test]
fn checkbox_default() {
    let img = render_widget_to_png(
        |cx| {
            let checked = signal(false);
            cx.add(checkbox(checked).class("bg-surface rounded-sm"))
        },
        InteractionMode::Default,
        [36, 36],
        (16.0, 16.0),
    );
    check("checkbox_default", img);
}

#[test]
fn checkbox_hover() {
    let img = render_widget_to_png(
        |cx| {
            let checked = signal(false);
            cx.add(checkbox(checked).class("bg-surface rounded-sm"))
        },
        InteractionMode::Hover,
        [36, 36],
        (16.0, 16.0),
    );
    check("checkbox_hover", img);
}

#[test]
fn checkbox_pressed() {
    let img = render_widget_to_png(
        |cx| {
            let checked = signal(false);
            cx.add(checkbox(checked).class("bg-surface rounded-sm"))
        },
        InteractionMode::Pressed,
        [36, 36],
        (16.0, 16.0),
    );
    check("checkbox_pressed", img);
}

#[test]
fn checkbox_focused() {
    let img = render_widget_to_png(
        |cx| {
            let checked = signal(false);
            cx.add(checkbox(checked).class("bg-surface rounded-sm"))
        },
        InteractionMode::Focused,
        [36, 36],
        (16.0, 16.0),
    );
    check("checkbox_focused", img);
}

#[test]
fn checkbox_disabled() {
    let img = render_widget_to_png(
        |cx| {
            let checked = signal(false);
            let dis = signal(true);
            cx.add(
                checkbox(checked)
                    .class("bg-surface rounded-sm")
                    .disabled(dis),
            )
        },
        InteractionMode::Disabled,
        [36, 36],
        (16.0, 16.0),
    );
    check("checkbox_disabled", img);
}

// ---- SLIDER ------------------------------------------------------------------

#[test]
fn slider_default() {
    let img = render_widget_to_png(
        |cx| {
            let v = signal(50.0_f32);
            cx.add(slider(v, 0.0..=100.0))
        },
        InteractionMode::Default,
        [192, 36],
        (90.0, 14.0),
    );
    check("slider_default", img);
}

#[test]
fn slider_hover() {
    let img = render_widget_to_png(
        |cx| {
            let v = signal(50.0_f32);
            cx.add(slider(v, 0.0..=100.0))
        },
        InteractionMode::Hover,
        [192, 36],
        (90.0, 14.0),
    );
    check("slider_hover", img);
}

#[test]
fn slider_pressed() {
    let img = render_widget_to_png(
        |cx| {
            let v = signal(50.0_f32);
            cx.add(slider(v, 0.0..=100.0))
        },
        InteractionMode::Pressed,
        [192, 36],
        (90.0, 14.0),
    );
    check("slider_pressed", img);
}

#[test]
fn slider_focused() {
    let img = render_widget_to_png(
        |cx| {
            let v = signal(50.0_f32);
            cx.add(slider(v, 0.0..=100.0))
        },
        InteractionMode::Focused,
        [192, 36],
        (90.0, 14.0),
    );
    check("slider_focused", img);
}

#[test]
fn slider_disabled() {
    let img = render_widget_to_png(
        |cx| {
            let v = signal(50.0_f32);
            let dis = signal(true);
            cx.add(slider(v, 0.0..=100.0).disabled(dis))
        },
        InteractionMode::Disabled,
        [192, 36],
        (90.0, 14.0),
    );
    check("slider_disabled", img);
}

// ---- PANEL -------------------------------------------------------------------
//
// Panel is a passive surface — it has no hover / pressed / focused / disabled
// states. We commit a single default golden.

#[test]
fn panel_default() {
    let img = render_widget_to_png(
        |cx| cx.add(panel().title("Settings")),
        InteractionMode::Default,
        [340, 92],
        (1.0, 1.0),
    );
    check("panel_default", img);
}

// ---- PROGRESS ----------------------------------------------------------------
//
// Progress is also passive — commit one golden per visually distinct fill
// level: empty, half, full.

#[test]
fn progress_empty() {
    let img = render_widget_to_png(
        |cx| {
            let v = signal(0.0_f32);
            cx.add(progress(v, 100.0).height(24.0))
        },
        InteractionMode::Default,
        [244, 32],
        (1.0, 1.0),
    );
    check("progress_empty", img);
}

#[test]
fn progress_half() {
    let img = render_widget_to_png(
        |cx| {
            let v = signal(50.0_f32);
            cx.add(progress(v, 100.0).height(24.0))
        },
        InteractionMode::Default,
        [244, 32],
        (1.0, 1.0),
    );
    check("progress_half", img);
}

#[test]
fn progress_full() {
    let img = render_widget_to_png(
        |cx| {
            let v = signal(100.0_f32);
            cx.add(progress(v, 100.0).height(24.0))
        },
        InteractionMode::Default,
        [244, 32],
        (1.0, 1.0),
    );
    check("progress_full", img);
}

// ---- DROPDOWN ----------------------------------------------------------------

fn dropdown_options() -> Vec<String> {
    vec!["Easy".into(), "Normal".into(), "Hard".into()]
}

#[test]
fn dropdown_default() {
    let img = render_widget_to_png(
        |cx| {
            let sel = signal(0_usize);
            cx.add(dropdown(sel, dropdown_options()))
        },
        InteractionMode::Default,
        [212, 52],
        (90.0, 22.0),
    );
    check("dropdown_default", img);
}

#[test]
fn dropdown_hover() {
    let img = render_widget_to_png(
        |cx| {
            let sel = signal(0_usize);
            cx.add(dropdown(sel, dropdown_options()))
        },
        InteractionMode::Hover,
        [212, 52],
        (90.0, 22.0),
    );
    check("dropdown_hover", img);
}

#[test]
fn dropdown_pressed() {
    let img = render_widget_to_png(
        |cx| {
            let sel = signal(0_usize);
            cx.add(dropdown(sel, dropdown_options()))
        },
        InteractionMode::Pressed,
        [212, 52],
        (90.0, 22.0),
    );
    check("dropdown_pressed", img);
}

#[test]
fn dropdown_focused() {
    let img = render_widget_to_png(
        |cx| {
            let sel = signal(0_usize);
            cx.add(dropdown(sel, dropdown_options()))
        },
        InteractionMode::Focused,
        [212, 52],
        (90.0, 22.0),
    );
    check("dropdown_focused", img);
}

#[test]
fn dropdown_disabled() {
    let img = render_widget_to_png(
        |cx| {
            let sel = signal(0_usize);
            let dis = signal(true);
            cx.add(dropdown(sel, dropdown_options()).disabled(dis))
        },
        InteractionMode::Disabled,
        [212, 52],
        (90.0, 22.0),
    );
    check("dropdown_disabled", img);
}

// ---- SCROLL ------------------------------------------------------------------
//
// Scroll wraps a content node; the only meaningful golden is the default
// composed view.

#[test]
fn scroll_default() {
    let img = render_widget_to_png(
        |cx| {
            let body = container()
                .class("bg-accent rounded-sm")
                .layout(LayoutStyle::block().fixed_size(312.0, 400.0));
            cx.add(scroll(body, 400.0).class("rounded-sm"))
        },
        InteractionMode::Default,
        [330, 252],
        (1.0, 1.0),
    );
    check("scroll_default", img);
}

// ---- TEXT_FIELD --------------------------------------------------------------

#[test]
fn text_field_default() {
    let img = render_widget_to_png(
        |cx| {
            let v = signal(String::from("Hello"));
            cx.add(text_field(v))
        },
        InteractionMode::Default,
        [192, 44],
        (40.0, 22.0),
    );
    check("text_field_default", img);
}

#[test]
fn text_field_hover() {
    let img = render_widget_to_png(
        |cx| {
            let v = signal(String::from("Hello"));
            cx.add(text_field(v))
        },
        InteractionMode::Hover,
        [192, 44],
        (40.0, 22.0),
    );
    check("text_field_hover", img);
}

#[test]
fn text_field_pressed() {
    let img = render_widget_to_png(
        |cx| {
            let v = signal(String::from("Hello"));
            cx.add(text_field(v))
        },
        InteractionMode::Pressed,
        [192, 44],
        (40.0, 22.0),
    );
    check("text_field_pressed", img);
}

#[test]
fn text_field_focused() {
    let img = render_widget_to_png(
        |cx| {
            let v = signal(String::from("Hello"));
            cx.add(text_field(v))
        },
        InteractionMode::Focused,
        [192, 44],
        (40.0, 22.0),
    );
    check("text_field_focused", img);
}

#[test]
fn text_field_disabled() {
    let img = render_widget_to_png(
        |cx| {
            let v = signal(String::from("Hello"));
            let dis = signal(true);
            cx.add(text_field(v).disabled(dis))
        },
        InteractionMode::Disabled,
        [192, 44],
        (40.0, 22.0),
    );
    check("text_field_disabled", img);
}
