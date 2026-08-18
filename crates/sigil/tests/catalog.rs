//! Component catalog captures for the GPUI comparison page — NOT
//! goldens. Each core widget rendered centered on a padded `bg-surface`
//! card at 2x scale, in both default themes, written to
//! `showcase/catalog/{light,dark}/<name>.png`. Run explicitly:
//! `cargo test -p sigil-ui --test catalog -- --ignored --test-threads=1`

#![allow(missing_docs)]
#![allow(unused_crate_dependencies)]
#![allow(clippy::unwrap_used, clippy::panic, clippy::print_stdout)]

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use sigil_app::DefaultTextMeasure;

mod golden_support;
use golden_support::{InteractionMode, render_widget_to_png_scaled};

use sigil::prelude::*;

fn install_fonts() {
    let assets = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../text/assets/fonts");
    let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../text/tests/fixtures");
    let sans = fs::read(assets.join("FiraSans-Regular.ttf")).expect("fira");
    let hebrew = fs::read(fixtures.join("NotoSansHebrew-Regular.ttf")).expect("noto hebrew");
    sigil::set_default_fonts(sigil::DefaultFonts {
        sans: Some((Arc::new(sans), 0)),
        mono: None,
        icon: Some((Arc::new(hebrew), 0)),
        display: None,
    });
}

fn out_dir(variant: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../showcase/catalog")
        .join(variant);
    fs::create_dir_all(&dir).expect("create catalog dir");
    dir
}

/// Render `build` on a padded surface card and save both theme variants.
fn shot(name: &str, logical: [u32; 2], build: impl Fn(&mut Cx, sigil_style::Theme) -> NodeId) {
    install_fonts();
    for (variant, theme) in [
        ("light", sigil_style::Theme::default()),
        ("dark", sigil_style::Theme::dark()),
    ] {
        let img = render_widget_to_png_scaled(
            |cx| build(cx, theme),
            InteractionMode::Default,
            logical,
            2.0,
            (1.0, 1.0),
            Box::new(DefaultTextMeasure::new()),
        );
        let Some(img) = img else {
            println!("no GPU adapter — skipped {name}");
            return;
        };
        let path = out_dir(variant).join(format!("{name}.png"));
        img.save(&path).expect("write catalog png");
        println!("wrote {}", path.display());
    }
}

/// Padded surface card that fills the capture canvas.
fn card(theme: sigil_style::Theme, inner: impl IntoNode + 'static) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_column()
                .fill()
                .padding(16.0)
                .justify_content(sigil::prelude::JustifyContent::Center),
        )
        .class("bg-surface")
        .theme(theme)
        .child(inner)
}

#[test]
#[ignore = "catalog capture — run explicitly"]
fn catalog() {
    catalog_controls();
    catalog_inputs();
}

/// Buttons, chips, dropdown, gauge, progress, radio.
fn catalog_controls() {
    shot("button", [400, 64], |cx, t| {
        cx.add(card(t, button("Save changes")))
    });
    shot("button_primary", [400, 64], |cx, t| {
        cx.add(card(t, button("Save changes").primary()))
    });
    shot("chips", [400, 64], |cx, t| {
        let row = container()
            .layout(LayoutStyle::flex_row().gap(8.0))
            .child(chip("SYNCED", Tone::Success))
            .child(chip("2 WARNINGS", Tone::Warn))
            .child(chip("FAILED", Tone::Error))
            .child(chip("v0.9.3", Tone::Neutral));
        cx.add(card(t, row))
    });
    shot("dropdown", [400, 68], |cx, t| {
        let selected = signal(1_usize);
        cx.add(card(
            t,
            dropdown(
                selected,
                vec!["Easy".into(), "Normal".into(), "Hard".into()],
            )
            .width(368.0),
        ))
    });
    shot("gauge", [400, 64], |cx, t| {
        let hp = signal(0.72_f32);
        cx.add(card(t, gauge(hp).max(1.0).label("HP").warn(0.3)))
    });
    shot("progress", [400, 56], |cx, t| {
        let v = signal(60.0_f32);
        cx.add(card(t, progress(v, 100.0).height(8.0)))
    });
    shot("radio_group", [400, 128], |cx, t| {
        let channel = signal(String::from("beta"));
        cx.add(card(
            t,
            radio_group(channel)
                .option("stable", "Stable")
                .option("beta", "Beta")
                .option("nightly", "Nightly"),
        ))
    });
}

/// Checkbox, slider, text field, toggle, tabs.
fn catalog_inputs() {
    shot("checkbox", [400, 64], |cx, t| {
        let checked = signal(true);
        cx.add(card(t, checkbox(checked).label("Enable notifications")))
    });
    shot("slider", [400, 56], |cx, t| {
        let v = signal(65.0_f32);
        cx.add(card(t, slider(v, 0.0..=100.0)))
    });
    shot("text_field", [400, 64], |cx, t| {
        let v = signal(String::from("Aldous Harding"));
        let wrapped = container()
            .layout(LayoutStyle::block().fixed_size(368.0, 32.0))
            .child(text_field(v));
        cx.add(card(t, wrapped))
    });
    shot("toggle", [400, 64], |cx, t| {
        let on = signal(true);
        cx.add(card(t, toggle(on)))
    });
    shot("tabs", [400, 64], |cx, t| {
        let active = signal(0_usize);
        cx.add(card(
            t,
            tabs(
                active,
                vec![
                    "General".to_owned(),
                    "Appearance".to_owned(),
                    "Keymap".to_owned(),
                ],
            ),
        ))
    });
}
