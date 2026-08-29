//! Component previews for the documentation site — NOT goldens. Each
//! widget renders on its own logical-pixel stage at 2x scale (tight
//! per-widget dimensions, see the shot calls), written to
//! `site/assets/components/<name>.png`. Run explicitly:
//! `cargo test -p sigil-ui --test catalog -- --ignored --test-threads=1`

#![allow(missing_docs)]
#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
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
    let medium = fs::read(assets.join("FiraSans-Medium.ttf")).expect("fira medium");
    sigil::set_default_fonts(sigil::DefaultFonts {
        sans: Some((Arc::new(sans), 0)),
        sans_weights: vec![(500, Arc::new(medium), 0)],
        fallbacks: vec![(Arc::new(hebrew), 0)],
        ..Default::default()
    });
}

/// Per-widget documentation stage. Small enough that the widget fills
/// the frame on the site; large enough for the widest intrinsics
/// (dropdown 320, gauge/text-field 420, slider 420).
fn out_dir() -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../site/assets/components");
    fs::create_dir_all(&dir).expect("create catalog dir");
    dir
}

/// Render `build` on the standard documentation stage.
fn shot(name: &str, stage: [u32; 2], build: impl Fn(&mut Cx, sigil_style::Theme) -> NodeId) {
    install_fonts();
    let img = render_widget_to_png_scaled(
        |cx| build(cx, sigil_style::Theme::default()),
        InteractionMode::Default,
        stage,
        2.0,
        (1.0, 1.0),
        Box::new(DefaultTextMeasure::new()),
    );
    let Some(img) = img else {
        println!("no GPU adapter — skipped {name}");
        return;
    };
    let path = out_dir().join(format!("{name}.png"));
    img.save(&path).expect("write catalog png");
    println!("wrote {}", path.display());
}

/// Padded surface card that fills the capture canvas.
fn card(theme: sigil_style::Theme, inner: impl IntoNode + 'static) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_column()
                .fill()
                .padding(32.0)
                .align_items(AlignItems::Center)
                .justify_content(JustifyContent::Center),
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
    shot("button_primary", [320, 120], |cx, t| {
        cx.add(card(t, button("Save changes").primary()))
    });
    shot("chips", [480, 120], |cx, t| {
        let row = container()
            .layout(LayoutStyle::flex_row().gap(8.0))
            .child(chip("SYNCED", Tone::Success))
            .child(chip("2 WARNINGS", Tone::Warn))
            .child(chip("FAILED", Tone::Error))
            .child(chip("v0.9.3", Tone::Neutral));
        cx.add(card(t, row))
    });
    shot("dropdown", [440, 140], |cx, t| {
        let selected = signal(1_usize);
        cx.add(card(
            t,
            dropdown(
                selected,
                vec!["Easy".into(), "Normal".into(), "Hard".into()],
            )
            .width(320.0),
        ))
    });
    shot("gauge", [500, 180], |cx, t| {
        let hp = signal(0.72_f32);
        let wrapped = container()
            .layout(LayoutStyle::block().fill_width())
            .child(gauge(hp).max(1.0).label("HP").warn(0.3));
        cx.add(card(t, wrapped))
    });
    shot("progress", [480, 100], |cx, t| {
        let v = signal(60.0_f32);
        let wrapped = container()
            .layout(LayoutStyle::block().fill_width())
            .child(progress(v, 100.0).height(8.0));
        cx.add(card(t, wrapped))
    });
    shot("radio_group", [380, 210], |cx, t| {
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
    shot("checkbox", [340, 110], |cx, t| {
        let checked = signal(true);
        let wrapped = container()
            .layout(LayoutStyle::block().fill_width())
            .child(checkbox(checked).label("Enable notifications"));
        cx.add(card(t, wrapped))
    });
    shot("slider", [480, 110], |cx, t| {
        let v = signal(65.0_f32);
        let wrapped = container()
            .layout(LayoutStyle::block().fill_width())
            .child(slider(v, 0.0..=100.0));
        cx.add(card(t, wrapped))
    });
    shot("text_field", [520, 130], |cx, t| {
        let v = signal(String::from("Aldous Harding"));
        let wrapped = container()
            .layout(LayoutStyle::block().fixed_size(420.0, 32.0))
            .child(text_field(v));
        cx.add(card(t, wrapped))
    });
    shot("toggle", [340, 110], |cx, t| {
        let on = signal(true);
        let row = container()
            .layout(
                LayoutStyle::flex_row()
                    .gap(12.0)
                    .align_items(AlignItems::Center),
            )
            .child(toggle(on))
            .child(text("Airplane mode").class("text-sm text-gray-12"));
        cx.add(card(t, row))
    });
    shot("tabs", [480, 150], |cx, t| {
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
