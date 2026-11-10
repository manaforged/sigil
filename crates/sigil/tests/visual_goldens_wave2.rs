#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]
#![allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
#![allow(clippy::cast_sign_loss, clippy::doc_markdown)]
#![allow(clippy::print_stdout, clippy::too_many_lines)]

use sigil::prelude::*;
use sigil::style::ControlSize;

use sigil_kittest::{Golden, Harness, InteractionMode};

const DELTA_PCT: f32 = 0.5;

#[test]
fn icon_default() {
    let img = Harness::render(|| {
        container()
            .background(Token::Surface)
            .layout(LayoutStyle::flex_row().fill())
            .child(icon("i"))
    })
    .stub()
    .mode(InteractionMode::Default)
    .size([32, 32])
    .hit((16.0, 16.0))
    .image();
    Golden::new("icon_default").delta(DELTA_PCT).assert(img);
}

#[test]
fn icon_primary_tone() {
    let img = Harness::render(|| {
        container()
            .background(Token::Surface)
            .layout(LayoutStyle::flex_row().fill())
            .child(icon("+").tone(Tone::Primary))
    })
    .stub()
    .mode(InteractionMode::Default)
    .size([32, 32])
    .hit((16.0, 16.0))
    .image();
    Golden::new("icon_primary_tone")
        .delta(DELTA_PCT)
        .assert(img);
}

#[test]
fn icon_error_tone() {
    let img = Harness::render(|| {
        container()
            .background(Token::Surface)
            .layout(LayoutStyle::flex_row().fill())
            .child(icon("x").tone(Tone::Error))
    })
    .stub()
    .mode(InteractionMode::Default)
    .size([32, 32])
    .hit((16.0, 16.0))
    .image();
    Golden::new("icon_error_tone").delta(DELTA_PCT).assert(img);
}

#[test]
fn dot_success() {
    let img = Harness::render(|| {
        container()
            .background(Token::Surface)
            .layout(LayoutStyle::flex_row().fill())
            .child(dot(Tone::Success))
    })
    .stub()
    .mode(InteractionMode::Default)
    .size([32, 32])
    .hit((16.0, 16.0))
    .image();
    Golden::new("dot_success").delta(DELTA_PCT).assert(img);
}

#[test]
fn dot_warn_default_size() {
    let img = Harness::render(|| {
        container()
            .background(Token::Surface)
            .layout(LayoutStyle::flex_row().fill())
            .child(dot(Tone::Warning).size(ControlSize::Default))
    })
    .stub()
    .mode(InteractionMode::Default)
    .size([32, 32])
    .hit((16.0, 16.0))
    .image();
    Golden::new("dot_warn_default_size")
        .delta(DELTA_PCT)
        .assert(img);
}

#[test]
fn dot_error_wide() {
    let img = Harness::render(|| {
        container()
            .background(Token::Surface)
            .layout(LayoutStyle::flex_row().fill())
            .child(dot(Tone::Error).size(ControlSize::Wide))
    })
    .stub()
    .mode(InteractionMode::Default)
    .size([32, 32])
    .hit((16.0, 16.0))
    .image();
    Golden::new("dot_error_wide").delta(DELTA_PCT).assert(img);
}

#[test]
fn radio_group_three_options_first_selected() {
    let img = Harness::render(|| {
        let selected = signal(String::from("light"));
        container()
            .background(Token::Surface)
            .layout(LayoutStyle::flex_row().fill())
            .child(
                radio_group(selected)
                    .option("light", "Light")
                    .option("dark", "Dark")
                    .option("auto", "Match system"),
            )
    })
    .stub()
    .mode(InteractionMode::Default)
    .size([220, 120])
    .hit((10.0, 14.0))
    .image();
    Golden::new("radio_group_three_options_first_selected")
        .delta(DELTA_PCT)
        .assert(img);
}

#[test]
fn radio_group_middle_selected() {
    let img = Harness::render(|| {
        let selected = signal(String::from("dark"));
        container()
            .background(Token::Surface)
            .layout(LayoutStyle::flex_row().fill())
            .child(
                radio_group(selected)
                    .option("light", "Light")
                    .option("dark", "Dark")
                    .option("auto", "Match system"),
            )
    })
    .stub()
    .mode(InteractionMode::Default)
    .size([220, 120])
    .hit((10.0, 42.0))
    .image();
    Golden::new("radio_group_middle_selected")
        .delta(DELTA_PCT)
        .assert(img);
}

#[test]
fn validated_field_no_error() {
    let img = Harness::render(|| {
        let error: Signal<Option<String>> = signal(None);
        container()
            .background(Token::Surface)
            .layout(LayoutStyle::flex_row().fill())
            .child(sigil::widgets::field("Email", text_field(signal(String::new()))).error(error))
    })
    .stub()
    .mode(InteractionMode::Default)
    .size([200, 52])
    .hit((10.0, 10.0))
    .image();
    Golden::new("validated_field_no_error")
        .delta(DELTA_PCT)
        .assert(img);
}

#[test]
fn validated_field_with_error() {
    let img = Harness::render(|| {
        let error: Signal<Option<String>> = signal(Some(String::from("Email must contain @")));
        container()
            .background(Token::Surface)
            .layout(LayoutStyle::flex_row().fill())
            .child(sigil::widgets::field("Email", text_field(signal(String::new()))).error(error))
    })
    .stub()
    .mode(InteractionMode::Default)
    .size([200, 80])
    .hit((10.0, 10.0))
    .image();
    Golden::new("validated_field_with_error")
        .delta(DELTA_PCT)
        .assert(img);
}
