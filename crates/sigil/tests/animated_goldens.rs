#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]
#![allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
#![allow(clippy::cast_sign_loss, clippy::doc_markdown)]
#![allow(clippy::print_stdout, clippy::too_many_lines)]

use sigil::prelude::*;

use sigil_kittest::{Golden, Harness, InteractionMode};

const DELTA_PCT: f32 = 0.5;
const DT: f32 = 1.0 / 60.0;

#[test]
fn button_hover_settled() {
    let img = Harness::render(|| button("Save"))
        .stub()
        .mode(InteractionMode::HoverAfterFrames {
            frames: u32::MAX,
            dt: DT,
        })
        .size([160, 56])
        .hit((24.0, 16.0))
        .image();
    Golden::new("button_hover_settled")
        .delta(DELTA_PCT)
        .assert(img);
}

#[test]
fn button_hover_midflight() {
    let img = Harness::render(|| button("Save"))
        .stub()
        .mode(InteractionMode::HoverAfterFrames { frames: 6, dt: DT })
        .size([160, 56])
        .hit((24.0, 16.0))
        .image();
    Golden::new("button_hover_midflight")
        .delta(DELTA_PCT)
        .assert(img);
}

#[test]
fn button_hover_filmstrip() {
    let img = Harness::render(|| button("Save"))
        .stub()
        .size([160, 56])
        .hit((24.0, 16.0))
        .filmstrip(6, DT);
    Golden::new("button_hover_filmstrip")
        .delta(DELTA_PCT)
        .assert(img);
}

fn checkbox_widget() -> impl IntoNode {
    let checked = signal(false);
    checkbox(checked).bg(Token::Surface).rounded(Radius::Sm)
}

#[test]
fn checkbox_hover_settled() {
    let img = Harness::render(checkbox_widget)
        .stub()
        .mode(InteractionMode::HoverAfterFrames {
            frames: u32::MAX,
            dt: DT,
        })
        .size([36, 36])
        .hit((16.0, 16.0))
        .image();
    Golden::new("checkbox_hover_settled")
        .delta(DELTA_PCT)
        .assert(img);
}

#[test]
fn checkbox_hover_midflight() {
    let img = Harness::render(checkbox_widget)
        .stub()
        .mode(InteractionMode::HoverAfterFrames { frames: 6, dt: DT })
        .size([36, 36])
        .hit((16.0, 16.0))
        .image();
    Golden::new("checkbox_hover_midflight")
        .delta(DELTA_PCT)
        .assert(img);
}

#[test]
fn checkbox_hover_filmstrip() {
    let img = Harness::render(checkbox_widget)
        .stub()
        .size([36, 36])
        .hit((16.0, 16.0))
        .filmstrip(6, DT);
    Golden::new("checkbox_hover_filmstrip")
        .delta(DELTA_PCT)
        .assert(img);
}

fn slider_widget() -> impl IntoNode {
    let v = signal(50.0_f32);
    slider(v, 0.0..=100.0)
}

#[test]
fn slider_hover_settled() {
    let img = Harness::render(slider_widget)
        .stub()
        .mode(InteractionMode::HoverAfterFrames {
            frames: u32::MAX,
            dt: DT,
        })
        .size([192, 36])
        .hit((90.0, 14.0))
        .image();
    Golden::new("slider_hover_settled")
        .delta(DELTA_PCT)
        .assert(img);
}

#[test]
fn slider_hover_midflight() {
    let img = Harness::render(slider_widget)
        .stub()
        .mode(InteractionMode::HoverAfterFrames { frames: 6, dt: DT })
        .size([192, 36])
        .hit((90.0, 14.0))
        .image();
    Golden::new("slider_hover_midflight")
        .delta(DELTA_PCT)
        .assert(img);
}

#[test]
fn slider_hover_filmstrip() {
    let img = Harness::render(slider_widget)
        .stub()
        .size([192, 36])
        .hit((90.0, 14.0))
        .filmstrip(6, DT);
    Golden::new("slider_hover_filmstrip")
        .delta(DELTA_PCT)
        .assert(img);
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
    let img = Harness::render(dropdown_widget)
        .stub()
        .mode(InteractionMode::HoverAfterFrames {
            frames: u32::MAX,
            dt: DT,
        })
        .size([212, 52])
        .hit((90.0, 22.0))
        .image();
    Golden::new("dropdown_hover_settled")
        .delta(DELTA_PCT)
        .assert(img);
}

#[test]
fn dropdown_hover_midflight() {
    let img = Harness::render(dropdown_widget)
        .stub()
        .mode(InteractionMode::HoverAfterFrames { frames: 6, dt: DT })
        .size([212, 52])
        .hit((90.0, 22.0))
        .image();
    Golden::new("dropdown_hover_midflight")
        .delta(DELTA_PCT)
        .assert(img);
}

#[test]
fn dropdown_hover_filmstrip() {
    let img = Harness::render(dropdown_widget)
        .stub()
        .size([212, 52])
        .hit((90.0, 22.0))
        .filmstrip(6, DT);
    Golden::new("dropdown_hover_filmstrip")
        .delta(DELTA_PCT)
        .assert(img);
}

fn text_field_widget() -> impl IntoNode {
    let v = signal(String::from("Hello"));
    text_field(v)
}

#[test]
fn text_field_hover_settled() {
    let img = Harness::render(text_field_widget)
        .stub()
        .mode(InteractionMode::HoverAfterFrames {
            frames: u32::MAX,
            dt: DT,
        })
        .size([192, 44])
        .hit((40.0, 22.0))
        .image();
    Golden::new("text_field_hover_settled")
        .delta(DELTA_PCT)
        .assert(img);
}

#[test]
fn text_field_hover_midflight() {
    let img = Harness::render(text_field_widget)
        .stub()
        .mode(InteractionMode::HoverAfterFrames { frames: 6, dt: DT })
        .size([192, 44])
        .hit((40.0, 22.0))
        .image();
    Golden::new("text_field_hover_midflight")
        .delta(DELTA_PCT)
        .assert(img);
}

#[test]
fn text_field_hover_filmstrip() {
    let img = Harness::render(text_field_widget)
        .stub()
        .size([192, 44])
        .hit((40.0, 22.0))
        .filmstrip(6, DT);
    Golden::new("text_field_hover_filmstrip")
        .delta(DELTA_PCT)
        .assert(img);
}

fn toggle_widget() -> impl IntoNode {
    let on = signal(false);
    toggle(on)
}

#[test]
fn toggle_hover_settled() {
    let img = Harness::render(toggle_widget)
        .stub()
        .mode(InteractionMode::HoverAfterFrames {
            frames: u32::MAX,
            dt: DT,
        })
        .size([50, 32])
        .hit((35.0, 22.4))
        .image();
    Golden::new("toggle_hover_settled")
        .delta(DELTA_PCT)
        .assert(img);
}

#[test]
fn toggle_hover_midflight() {
    let img = Harness::render(toggle_widget)
        .stub()
        .mode(InteractionMode::HoverAfterFrames { frames: 6, dt: DT })
        .size([50, 32])
        .hit((35.0, 22.4))
        .image();
    Golden::new("toggle_hover_midflight")
        .delta(DELTA_PCT)
        .assert(img);
}

#[test]
fn toggle_hover_filmstrip() {
    let img = Harness::render(toggle_widget)
        .stub()
        .size([50, 32])
        .hit((35.0, 22.4))
        .filmstrip(6, DT);
    Golden::new("toggle_hover_filmstrip")
        .delta(DELTA_PCT)
        .assert(img);
}
