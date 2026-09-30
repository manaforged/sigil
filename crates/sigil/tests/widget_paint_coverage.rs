#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used, clippy::panic, clippy::print_stdout)]

use std::collections::HashSet;

use sigil::prelude::*;
use sigil::style::Rgba8;
use sigil::widgets::{
    Combobox, Date, TableColumn, TableState, badge, chart, checkbox, chip, code_edit, color_field,
    combobox, date_field, divider_horizontal, dot, dropdown, gauge, icon_button, markdown,
    number_field, progress, radio_group, scroll, slider, table, tabs, text_edit, text_field,
    toggle, virtual_list,
};
use sigil_kittest::{InteractionMode, render_widget_to_png_stub};

type Case = (&'static str, Box<dyn IntoNode>);

struct PaintRow {
    name: &'static str,
    lang: &'static str,
}

fn columns() -> Vec<TableColumn<PaintRow>> {
    vec![
        TableColumn::new("name", "Name").cell(|row: &PaintRow| text(row.name).nowrap()),
        TableColumn::new("lang", "Language").cell(|row: &PaintRow| text(row.lang).nowrap()),
    ]
}

fn controls() -> Vec<Case> {
    vec![
        ("button", Box::new(|| button("Save").primary())),
        (
            "icon_button",
            Box::new(|| icon_button("R", || {}).accessible("Reset")),
        ),
        ("chip", Box::new(|| chip("Stable", Tone::Success))),
        ("badge", Box::new(|| badge("3", Tone::Info))),
        (
            "checkbox",
            Box::new(|| checkbox(signal(true)).label("Enabled")),
        ),
        ("toggle", Box::new(|| toggle(signal(true)).label("Lock"))),
        (
            "radio_group",
            Box::new(|| {
                radio_group(signal(String::from("a")))
                    .option("a", "Automatic")
                    .option("m", "Manual")
            }),
        ),
        ("slider", Box::new(|| slider(signal(40.0_f32), 0.0..=100.0))),
        (
            "gauge",
            Box::new(|| gauge(signal(40.0_f32)).max(100.0).label("Battery")),
        ),
        ("progress", Box::new(|| progress(signal(40.0_f32), 100.0))),
    ]
}

fn inputs() -> Vec<Case> {
    vec![
        (
            "text_field",
            Box::new(|| text_field(signal(String::from("abc")))),
        ),
        (
            "text_edit",
            Box::new(|| text_edit(signal(String::from("line\nline")))),
        ),
        (
            "number_field",
            Box::new(|| number_field(signal(4_i32), 1..=99)),
        ),
        (
            "date_field",
            Box::new(|| {
                date_field(signal(Date {
                    year: 2026,
                    month: 4,
                    day: 1,
                }))
            }),
        ),
        (
            "color_field",
            Box::new(|| {
                color_field(signal(Rgba8 {
                    r: 128,
                    g: 64,
                    b: 200,
                    a: 255,
                }))
            }),
        ),
    ]
}

fn pickers() -> Vec<Case> {
    vec![
        (
            "dropdown",
            Box::new(|| {
                dropdown(
                    signal(1_usize),
                    vec!["Small".into(), "Medium".into(), "Large".into()],
                )
            }),
        ),
        (
            "combobox",
            Box::new(|| {
                combobox(
                    Combobox::new(vec![String::from("Rust"), String::from("Go")])
                        .placeholder("Language"),
                )
            }),
        ),
    ]
}

fn data() -> Vec<Case> {
    vec![
        ("dot", Box::new(|| dot(Tone::Warn))),
        ("divider", Box::new(divider_horizontal)),
        ("label", Box::new(|| text("Field").nowrap())),
        ("text", Box::new(|| text("Body copy"))),
        (
            "markdown",
            Box::new(|| markdown(signal(String::from("# Head\n\nBody")))),
        ),
        (
            "scroll",
            Box::new(|| {
                scroll(container().child(text("top")).child(text("bottom"))).extent(400.0_f32)
            }),
        ),
        ("tabs", Box::new(|| tabs().labels(["One", "Two"]))),
    ]
}

fn views() -> Vec<Case> {
    vec![
        (
            "table",
            Box::new(|| {
                table(TableState::new(
                    vec![
                        PaintRow {
                            name: "sigil",
                            lang: "Rust",
                        },
                        PaintRow {
                            name: "wgpu",
                            lang: "Rust",
                        },
                    ],
                    columns(),
                ))
                .viewport_height(120.0)
            }),
        ),
        (
            "virtual_list",
            Box::new(|| {
                virtual_list(vec!["a".to_string(), "b".to_string()], |_, item| {
                    container().child(text(item.clone()).nowrap())
                })
                .row_height(24.0)
                .viewport_height(80.0)
                .viewport_width(200.0)
            }),
        ),
        ("chart", Box::new(|| chart(vec![1.0_f32, 4.0, 2.0, 6.0]))),
        (
            "code_edit",
            Box::new(|| {
                code_edit(
                    signal(vec![String::from("fn main() {}")]),
                    signal((0_usize, 0_usize)),
                )
            }),
        ),
    ]
}

fn assert_painted(stem: &str, img: Option<image::RgbaImage>) {
    let Some(img) = img else {
        println!("no GPU adapter — skipping {stem}");
        return;
    };
    let distinct: HashSet<_> = img.pixels().map(|p| p.0).collect();
    assert!(
        distinct.len() > 1,
        "{stem} rendered a flat single-color frame — nothing painted"
    );
}

fn run(cases: Vec<Case>) {
    for (stem, build) in cases {
        assert_painted(
            stem,
            render_widget_to_png_stub(build, InteractionMode::Default, [240, 96], (0.0, 0.0))
                .expect("sigil-kittest"),
        );
    }
}

#[test]
fn controls_paint_pixels() {
    run(controls());
}

#[test]
fn inputs_paint_pixels() {
    run(inputs());
}

#[test]
fn pickers_paint_pixels() {
    run(pickers());
}

#[test]
fn data_widgets_paint_pixels() {
    run(data());
}

#[test]
fn data_views_paint_pixels() {
    run(views());
}
