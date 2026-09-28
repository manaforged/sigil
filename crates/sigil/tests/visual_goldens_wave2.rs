#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]
#![allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
#![allow(clippy::cast_sign_loss, clippy::doc_markdown)]
#![allow(clippy::print_stdout, clippy::too_many_lines)]

use sigil::prelude::*;
use sigil::style::Size as ControlSize;

use sigil_kittest::{
    InteractionMode, assert_rendered_matches_golden, golden_path, render_widget_to_png_stub,
};

const DELTA_PCT: f32 = 0.5;

#[test]
fn icon_default() {
    let img = render_widget_to_png_stub(
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
    assert_rendered_matches_golden(img, &golden_path!("icon_default"), DELTA_PCT);
}

#[test]
fn icon_primary_tone() {
    let img = render_widget_to_png_stub(
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
    assert_rendered_matches_golden(img, &golden_path!("icon_primary_tone"), DELTA_PCT);
}

#[test]
fn icon_error_tone() {
    let img = render_widget_to_png_stub(
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
    assert_rendered_matches_golden(img, &golden_path!("icon_error_tone"), DELTA_PCT);
}

#[test]
fn dot_success() {
    let img = render_widget_to_png_stub(
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
    assert_rendered_matches_golden(img, &golden_path!("dot_success"), DELTA_PCT);
}

#[test]
fn dot_warn_default_size() {
    let img = render_widget_to_png_stub(
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
    assert_rendered_matches_golden(img, &golden_path!("dot_warn_default_size"), DELTA_PCT);
}

#[test]
fn dot_error_wide() {
    let img = render_widget_to_png_stub(
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
    assert_rendered_matches_golden(img, &golden_path!("dot_error_wide"), DELTA_PCT);
}

#[test]
fn radio_group_three_options_first_selected() {
    let img = render_widget_to_png_stub(
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
    assert_rendered_matches_golden(
        img,
        &golden_path!("radio_group_three_options_first_selected"),
        DELTA_PCT,
    );
}

#[test]
fn radio_group_middle_selected() {
    let img = render_widget_to_png_stub(
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
    assert_rendered_matches_golden(img, &golden_path!("radio_group_middle_selected"), DELTA_PCT);
}

#[test]
fn validated_field_no_error() {
    let img = render_widget_to_png_stub(
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
    assert_rendered_matches_golden(img, &golden_path!("validated_field_no_error"), DELTA_PCT);
}

#[test]
fn validated_field_with_error() {
    let img = render_widget_to_png_stub(
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
    assert_rendered_matches_golden(img, &golden_path!("validated_field_with_error"), DELTA_PCT);
}
