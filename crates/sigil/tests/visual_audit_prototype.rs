#![allow(
    missing_docs,
    unused_crate_dependencies,
    clippy::unwrap_used,
    clippy::print_stdout
)]

use std::fs;

mod golden_support;

use golden_support::{InteractionMode, render_widget_to_png_with};
use sigil::prelude::*;
use sigil_app::DefaultTextMeasure;

fn save(stem: &str, img: Option<image::RgbaImage>) {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/output/audit");
    fs::create_dir_all(dir).unwrap();
    let path = format!("{dir}/{stem}.png");
    match img {
        Some(img) => {
            img.save(&path).unwrap();
            println!("saved {path}");
        }
        None => println!("NO ADAPTER — skipped {stem}"),
    }
}

fn render(stem: &str, mode: InteractionMode, build: impl FnOnce(&mut Cx) -> NodeId) {
    let img = render_widget_to_png_with(
        build,
        mode,
        [360, 96],
        (40.0, 14.0),
        Box::new(DefaultTextMeasure::new()),
    );
    save(stem, img);
}

#[test]
fn audit_row_defaults() {
    let img = render_widget_to_png_with(
        |cx| {
            let dd = signal(1_usize);
            cx.add(
                container()
                    .layout(LayoutStyle::flex_row().gap(16.0).padding(16.0))
                    .class("bg-surface")
                    .child(button("Save").primary())
                    .child(button("Cancel"))
                    .child(chip("Success", Tone::Success))
                    .child(chip("Warn", Tone::Warn))
                    .child(chip("Error", Tone::Error))
                    .child(dropdown(dd, vec!["One".into(), "Two".into()])),
            )
        },
        InteractionMode::Default,
        [720, 96],
        (12.0, 12.0),
        Box::new(DefaultTextMeasure::new()),
    );
    save("row_defaults", img);
}

#[test]
fn audit_button_states() {
    for mode in [
        InteractionMode::Default,
        InteractionMode::Hover,
        InteractionMode::Pressed,
        InteractionMode::Focused,
        InteractionMode::Disabled,
    ] {
        let tag = mode.tag();
        let mode_clone = mode;
        render(&format!("button_primary_{tag}"), mode_clone, |cx| {
            cx.add(button("Save").primary())
        });
        render(&format!("button_soft_{tag}"), mode_clone, |cx| {
            cx.add(button("Cancel"))
        });
    }
}

#[test]
fn audit_text_field_states() {
    for mode in [
        InteractionMode::Default,
        InteractionMode::Focused,
        InteractionMode::Disabled,
    ] {
        let tag = mode.tag();
        let m = mode;
        render(&format!("text_field_{tag}"), m, |cx| {
            let v = signal(String::from("hello@shop.com"));
            cx.add(text_field(v).placeholder("email"))
        });
    }
}

#[test]
fn audit_toggle_checkbox() {
    render("toggle_default", InteractionMode::Default, |cx| {
        let on = signal(true);
        cx.add(toggle(on).label("Notifications"))
    });
    render("checkbox_default", InteractionMode::Default, |cx| {
        let on = signal(true);
        cx.add(checkbox(on).label("Remember proxy"))
    });
}

#[test]
fn audit_dropdown_states() {
    for mode in [InteractionMode::Default, InteractionMode::Hover] {
        let tag = mode.tag();
        let m = mode;
        render(&format!("dropdown_{tag}"), m, |cx| {
            let sel = signal(1_usize);
            cx.add(dropdown(sel, vec!["Apple".into(), "Banana".into()]))
        });
    }
}
