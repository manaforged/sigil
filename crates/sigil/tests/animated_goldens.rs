#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]
#![allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
#![allow(clippy::cast_sign_loss, clippy::doc_markdown)]
#![allow(clippy::print_stdout, clippy::too_many_lines)]

use sigil::prelude::*;

use sigil_kittest::{
    InteractionMode, assert_rendered_matches_golden, capture_filmstrip, golden_path,
    render_widget_to_png_stub,
};

const DELTA_PCT: f32 = 0.5;
const DT: f32 = 1.0 / 60.0;

#[test]
fn button_hover_settled() {
    let img = render_widget_to_png_stub(
        || button("Save"),
        InteractionMode::HoverAfterFrames {
            frames: u32::MAX,
            dt: DT,
        },
        [160, 56],
        (24.0, 16.0),
    );
    assert_rendered_matches_golden(img, &golden_path!("button_hover_settled"), DELTA_PCT);
}

#[test]
fn button_hover_midflight() {
    let img = render_widget_to_png_stub(
        || button("Save"),
        InteractionMode::HoverAfterFrames { frames: 6, dt: DT },
        [160, 56],
        (24.0, 16.0),
    );
    assert_rendered_matches_golden(img, &golden_path!("button_hover_midflight"), DELTA_PCT);
}

#[test]
fn button_hover_filmstrip() {
    let img = capture_filmstrip(
        || button("Save"),
        [160, 56],
        (24.0, 16.0),
        6,
        DT,
        sigil_kittest::stub_text_measure,
    );
    assert_rendered_matches_golden(img, &golden_path!("button_hover_filmstrip"), DELTA_PCT);
}

fn checkbox_widget() -> impl IntoNode {
    let checked = signal(false);
    checkbox(checked).bg(Token::Surface).rounded(Radius::Sm)
}

#[test]
fn checkbox_hover_settled() {
    let img = render_widget_to_png_stub(
        checkbox_widget,
        InteractionMode::HoverAfterFrames {
            frames: u32::MAX,
            dt: DT,
        },
        [36, 36],
        (16.0, 16.0),
    );
    assert_rendered_matches_golden(img, &golden_path!("checkbox_hover_settled"), DELTA_PCT);
}

#[test]
fn checkbox_hover_midflight() {
    let img = render_widget_to_png_stub(
        checkbox_widget,
        InteractionMode::HoverAfterFrames { frames: 6, dt: DT },
        [36, 36],
        (16.0, 16.0),
    );
    assert_rendered_matches_golden(img, &golden_path!("checkbox_hover_midflight"), DELTA_PCT);
}

#[test]
fn checkbox_hover_filmstrip() {
    let img = capture_filmstrip(
        checkbox_widget,
        [36, 36],
        (16.0, 16.0),
        6,
        DT,
        sigil_kittest::stub_text_measure,
    );
    assert_rendered_matches_golden(img, &golden_path!("checkbox_hover_filmstrip"), DELTA_PCT);
}

fn slider_widget() -> impl IntoNode {
    let v = signal(50.0_f32);
    slider(v, 0.0..=100.0)
}

#[test]
fn slider_hover_settled() {
    let img = render_widget_to_png_stub(
        slider_widget,
        InteractionMode::HoverAfterFrames {
            frames: u32::MAX,
            dt: DT,
        },
        [192, 36],
        (90.0, 14.0),
    );
    assert_rendered_matches_golden(img, &golden_path!("slider_hover_settled"), DELTA_PCT);
}

#[test]
fn slider_hover_midflight() {
    let img = render_widget_to_png_stub(
        slider_widget,
        InteractionMode::HoverAfterFrames { frames: 6, dt: DT },
        [192, 36],
        (90.0, 14.0),
    );
    assert_rendered_matches_golden(img, &golden_path!("slider_hover_midflight"), DELTA_PCT);
}

#[test]
fn slider_hover_filmstrip() {
    let img = capture_filmstrip(
        slider_widget,
        [192, 36],
        (90.0, 14.0),
        6,
        DT,
        sigil_kittest::stub_text_measure,
    );
    assert_rendered_matches_golden(img, &golden_path!("slider_hover_filmstrip"), DELTA_PCT);
}

fn dropdown_widget_options() -> Vec<String> {
    vec![String::from("First"), String::from("Second")]
}

fn dropdown_widget() -> impl IntoNode {
    let sel = signal(0_usize);
    dropdown(sel, dropdown_widget_options())
}

#[test]
fn dropdown_hover_settled() {
    let img = render_widget_to_png_stub(
        dropdown_widget,
        InteractionMode::HoverAfterFrames {
            frames: u32::MAX,
            dt: DT,
        },
        [212, 52],
        (90.0, 22.0),
    );
    assert_rendered_matches_golden(img, &golden_path!("dropdown_hover_settled"), DELTA_PCT);
}

#[test]
fn dropdown_hover_midflight() {
    let img = render_widget_to_png_stub(
        dropdown_widget,
        InteractionMode::HoverAfterFrames { frames: 6, dt: DT },
        [212, 52],
        (90.0, 22.0),
    );
    assert_rendered_matches_golden(img, &golden_path!("dropdown_hover_midflight"), DELTA_PCT);
}

#[test]
fn dropdown_hover_filmstrip() {
    let img = capture_filmstrip(
        dropdown_widget,
        [212, 52],
        (90.0, 22.0),
        6,
        DT,
        sigil_kittest::stub_text_measure,
    );
    assert_rendered_matches_golden(img, &golden_path!("dropdown_hover_filmstrip"), DELTA_PCT);
}

fn text_field_widget() -> impl IntoNode {
    let v = signal(String::from("Hello"));
    text_field(v)
}

#[test]
fn text_field_hover_settled() {
    let img = render_widget_to_png_stub(
        text_field_widget,
        InteractionMode::HoverAfterFrames {
            frames: u32::MAX,
            dt: DT,
        },
        [192, 44],
        (40.0, 22.0),
    );
    assert_rendered_matches_golden(img, &golden_path!("text_field_hover_settled"), DELTA_PCT);
}

#[test]
fn text_field_hover_midflight() {
    let img = render_widget_to_png_stub(
        text_field_widget,
        InteractionMode::HoverAfterFrames { frames: 6, dt: DT },
        [192, 44],
        (40.0, 22.0),
    );
    assert_rendered_matches_golden(img, &golden_path!("text_field_hover_midflight"), DELTA_PCT);
}

#[test]
fn text_field_hover_filmstrip() {
    let img = capture_filmstrip(
        text_field_widget,
        [192, 44],
        (40.0, 22.0),
        6,
        DT,
        sigil_kittest::stub_text_measure,
    );
    assert_rendered_matches_golden(img, &golden_path!("text_field_hover_filmstrip"), DELTA_PCT);
}

fn toggle_widget() -> impl IntoNode {
    let on = signal(false);
    toggle(on)
}

#[test]
fn toggle_hover_settled() {
    let img = render_widget_to_png_stub(
        toggle_widget,
        InteractionMode::HoverAfterFrames {
            frames: u32::MAX,
            dt: DT,
        },
        [50, 32],
        (35.0, 22.4),
    );
    assert_rendered_matches_golden(img, &golden_path!("toggle_hover_settled"), DELTA_PCT);
}

#[test]
fn toggle_hover_midflight() {
    let img = render_widget_to_png_stub(
        toggle_widget,
        InteractionMode::HoverAfterFrames { frames: 6, dt: DT },
        [50, 32],
        (35.0, 22.4),
    );
    assert_rendered_matches_golden(img, &golden_path!("toggle_hover_midflight"), DELTA_PCT);
}

#[test]
fn toggle_hover_filmstrip() {
    let img = capture_filmstrip(
        toggle_widget,
        [50, 32],
        (35.0, 22.4),
        6,
        DT,
        sigil_kittest::stub_text_measure,
    );
    assert_rendered_matches_golden(img, &golden_path!("toggle_hover_filmstrip"), DELTA_PCT);
}
