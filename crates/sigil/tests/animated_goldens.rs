#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
//! Animated state-transition goldens for sigil widgets.
//!
//! Three goldens per animated widget:
//! - `<widget>_hover_settled.png` — `HoverAfterFrames { u32::MAX }`,
//!   transition has fully landed at the hover endpoint.
//! - `<widget>_hover_midflight.png` — `HoverAfterFrames { 6 }`, captures
//!   the curve mid-animation (~100 ms in at 60 Hz).
//! - `<widget>_hover_filmstrip.png` — 6 frames horizontally in one PNG,
//!   the full visual review artifact for the entire animation curve.

#![allow(missing_docs)]
#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]
#![allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
#![allow(clippy::cast_sign_loss, clippy::doc_markdown)]
#![allow(clippy::print_stdout, clippy::too_many_lines)]

use sigil::prelude::*;

mod golden_support;

use golden_support::{
    InteractionMode, assert_png_matches_golden, capture_filmstrip, golden_path,
    render_widget_to_png,
};

const DELTA_PCT: f32 = 0.5;
const DT: f32 = 1.0 / 60.0;

fn check(stem: &str, img: Option<image::RgbaImage>) {
    let Some(img) = img else {
        println!("no GPU adapter — skipping {stem}");
        return;
    };
    assert_png_matches_golden(&img, &golden_path(stem), DELTA_PCT);
}

// ---- BUTTON ------------------------------------------------------------------

#[test]
fn button_hover_settled() {
    let img = render_widget_to_png(
        |cx| cx.add(button("Save")),
        InteractionMode::HoverAfterFrames {
            frames: u32::MAX,
            dt: DT,
        },
        [160, 56],
        (24.0, 16.0),
    );
    check("button_hover_settled", img);
}

#[test]
fn button_hover_midflight() {
    let img = render_widget_to_png(
        |cx| cx.add(button("Save")),
        InteractionMode::HoverAfterFrames { frames: 6, dt: DT },
        [160, 56],
        (24.0, 16.0),
    );
    check("button_hover_midflight", img);
}

#[test]
fn button_hover_filmstrip() {
    let img = capture_filmstrip(|cx| cx.add(button("Save")), [160, 56], (24.0, 16.0), 6, DT);
    check("button_hover_filmstrip", img);
}

// ---- CHECKBOX ----------------------------------------------------------------

fn checkbox_widget(cx: &mut Cx) -> NodeId {
    let checked = signal(false);
    cx.add(checkbox(checked).class("bg-surface rounded-sm"))
}

#[test]
fn checkbox_hover_settled() {
    let img = render_widget_to_png(
        checkbox_widget,
        InteractionMode::HoverAfterFrames {
            frames: u32::MAX,
            dt: DT,
        },
        [36, 36],
        (16.0, 16.0),
    );
    check("checkbox_hover_settled", img);
}

#[test]
fn checkbox_hover_midflight() {
    let img = render_widget_to_png(
        checkbox_widget,
        InteractionMode::HoverAfterFrames { frames: 6, dt: DT },
        [36, 36],
        (16.0, 16.0),
    );
    check("checkbox_hover_midflight", img);
}

#[test]
fn checkbox_hover_filmstrip() {
    let img = capture_filmstrip(checkbox_widget, [36, 36], (16.0, 16.0), 6, DT);
    check("checkbox_hover_filmstrip", img);
}

// ---- SLIDER ------------------------------------------------------------------

fn slider_widget(cx: &mut Cx) -> NodeId {
    let v = signal(50.0_f32);
    cx.add(slider(v, 0.0..=100.0))
}

#[test]
fn slider_hover_settled() {
    let img = render_widget_to_png(
        slider_widget,
        InteractionMode::HoverAfterFrames {
            frames: u32::MAX,
            dt: DT,
        },
        [192, 36],
        (90.0, 14.0),
    );
    check("slider_hover_settled", img);
}

#[test]
fn slider_hover_midflight() {
    let img = render_widget_to_png(
        slider_widget,
        InteractionMode::HoverAfterFrames { frames: 6, dt: DT },
        [192, 36],
        (90.0, 14.0),
    );
    check("slider_hover_midflight", img);
}

#[test]
fn slider_hover_filmstrip() {
    let img = capture_filmstrip(slider_widget, [192, 36], (90.0, 14.0), 6, DT);
    check("slider_hover_filmstrip", img);
}

// ---- DROPDOWN ----------------------------------------------------------------

fn dropdown_widget_options() -> Vec<String> {
    vec![String::from("First"), String::from("Second")]
}

fn dropdown_widget(cx: &mut Cx) -> NodeId {
    let sel = signal(0_usize);
    cx.add(dropdown(sel, dropdown_widget_options()))
}

#[test]
fn dropdown_hover_settled() {
    let img = render_widget_to_png(
        dropdown_widget,
        InteractionMode::HoverAfterFrames {
            frames: u32::MAX,
            dt: DT,
        },
        [212, 52],
        (90.0, 22.0),
    );
    check("dropdown_hover_settled", img);
}

#[test]
fn dropdown_hover_midflight() {
    let img = render_widget_to_png(
        dropdown_widget,
        InteractionMode::HoverAfterFrames { frames: 6, dt: DT },
        [212, 52],
        (90.0, 22.0),
    );
    check("dropdown_hover_midflight", img);
}

#[test]
fn dropdown_hover_filmstrip() {
    let img = capture_filmstrip(dropdown_widget, [212, 52], (90.0, 22.0), 6, DT);
    check("dropdown_hover_filmstrip", img);
}

// ---- TEXT_FIELD --------------------------------------------------------------

fn text_field_widget(cx: &mut Cx) -> NodeId {
    let v = signal(String::from("Hello"));
    cx.add(text_field(v))
}

#[test]
fn text_field_hover_settled() {
    let img = render_widget_to_png(
        text_field_widget,
        InteractionMode::HoverAfterFrames {
            frames: u32::MAX,
            dt: DT,
        },
        [192, 44],
        (40.0, 22.0),
    );
    check("text_field_hover_settled", img);
}

#[test]
fn text_field_hover_midflight() {
    let img = render_widget_to_png(
        text_field_widget,
        InteractionMode::HoverAfterFrames { frames: 6, dt: DT },
        [192, 44],
        (40.0, 22.0),
    );
    check("text_field_hover_midflight", img);
}

#[test]
fn text_field_hover_filmstrip() {
    let img = capture_filmstrip(text_field_widget, [192, 44], (40.0, 22.0), 6, DT);
    check("text_field_hover_filmstrip", img);
}

// ---- TOGGLE ------------------------------------------------------------------

fn toggle_widget(cx: &mut Cx) -> NodeId {
    let on = signal(false);
    cx.add(toggle(on))
}

#[test]
fn toggle_hover_settled() {
    let img = render_widget_to_png(
        toggle_widget,
        InteractionMode::HoverAfterFrames {
            frames: u32::MAX,
            dt: DT,
        },
        [50, 32],
        (35.0, 22.4),
    );
    check("toggle_hover_settled", img);
}

#[test]
fn toggle_hover_midflight() {
    let img = render_widget_to_png(
        toggle_widget,
        InteractionMode::HoverAfterFrames { frames: 6, dt: DT },
        [50, 32],
        (35.0, 22.4),
    );
    check("toggle_hover_midflight", img);
}

#[test]
fn toggle_hover_filmstrip() {
    // Per-frame box tight to the toggle's natural extent (matches the
    // settled/midflight goldens' [50, 32] frame) so each stitched frame
    // is dominated by the widget, not empty canvas. A 200×125 box left
    // the ~44×24 toggle filling < 6 % of the frame.
    let img = capture_filmstrip(toggle_widget, [50, 32], (35.0, 22.4), 6, DT);
    check("toggle_hover_filmstrip", img);
}
