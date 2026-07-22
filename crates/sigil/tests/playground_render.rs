//! Render the widget-playground composition to a single 1100×800 PNG
//! at `crates/sigil/tests/output/playground.png`.
//!
//! Not a byte-equivalence golden — the playground is iterating. This
//! exists so a developer / agent session can inspect what the live
//! composition actually paints after every layout-engine change.

#![allow(missing_docs)]
#![allow(unused_crate_dependencies)]
#![allow(clippy::unwrap_used, clippy::panic, clippy::print_stdout)]

use std::fs;
use std::path::PathBuf;

use arboard as _;
use criterion as _;
use image as _;
use sigil_actions as _;
use sigil_animation as _;
use sigil_animation_runtime as _;
use sigil_app as _;
use sigil_bidi as _;
use sigil_events as _;
use sigil_hal as _;
use sigil_kernel as _;
use sigil_layout as _;
use sigil_reactive as _;
use sigil_renderer as _;
use sigil_style as _;
use sigil_tasks as _;
use wgpu as _;

mod golden_support;
use golden_support::{InteractionMode, render_widget_to_png_with};
use sigil_app::DefaultTextMeasure;

// Mount the playground section files verbatim. Each `use crate::{…}`
// inside them resolves against the helpers defined at the root of
// THIS test crate, below.
#[path = "../../../examples/widget-playground/src/section_container.rs"]
mod section_container;
#[path = "../../../examples/widget-playground/src/section_display.rs"]
mod section_display;
#[path = "../../../examples/widget-playground/src/section_form.rs"]
mod section_form;
#[path = "../../../examples/widget-playground/src/section_navigation.rs"]
mod section_navigation;
#[path = "../../../examples/widget-playground/src/section_reactive.rs"]
mod section_reactive;

use sigil::prelude::*;
use sigil::widgets::scroll;

use crate::section_container::container_section;
use crate::section_display::display_section;
use crate::section_form::form_section;
use crate::section_navigation::navigation_section;
use crate::section_reactive::reactive_section;

const SECTION_GAP: f32 = 24.0;
const PAGE_PADDING: f32 = 16.0;
const CONTENT_HEIGHT: f32 = 2400.0;
const VIEWPORT_WIDTH: f32 = 1080.0;
const VIEWPORT_HEIGHT: f32 = 760.0;

fn root() -> NodeBuilder {
    let content = container()
        .role(Role::Main)
        .label("Widget playground")
        .layout(
            LayoutStyle::flex_column()
                .padding(PAGE_PADDING)
                .gap(SECTION_GAP),
        )
        .class("w-full bg-surface")
        .child(page_header())
        .child(form_section())
        .child(display_section())
        .child(navigation_section())
        .child(container_section())
        .child(reactive_section());

    let viewport = scroll(content, CONTENT_HEIGHT)
        .width(VIEWPORT_WIDTH)
        .height(VIEWPORT_HEIGHT);
    container()
        .layout(LayoutStyle::flex_column().fill())
        .class("bg-surface")
        .child(viewport)
}

fn page_header() -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_column().gap(4.0))
        .child(text("Sigil widget playground").class("text-xl font-bold"))
        .child(text("Every widget the library exposes, one instance each.").class("text-sm"))
}

pub(crate) fn category(title: &'static str, body: NodeBuilder) -> NodeBuilder {
    let card = container()
        .layout(LayoutStyle::flex_column().gap(12.0).padding(16.0))
        .class("w-full rounded-md bg-surface-hover")
        .child(body);
    container()
        .layout(LayoutStyle::flex_column().fill_width().gap(8.0))
        .child(text(title).class("text-lg font-bold"))
        .child(card)
}

pub(crate) fn row(children: Vec<NodeBuilder>) -> NodeBuilder {
    let mut c = container().layout(
        LayoutStyle::flex_row()
            .fill_width()
            .gap(12.0)
            .align_items(AlignItems::Start),
    );
    for child in children {
        c = c.child(
            container()
                .layout(LayoutStyle::flex_column().grow(1.0).width(0.0))
                .child(child),
        );
    }
    c
}

pub(crate) fn column(children: Vec<NodeBuilder>) -> NodeBuilder {
    let mut c = container().layout(LayoutStyle::flex_column().gap(8.0).fill_width());
    for child in children {
        c = c.child(child);
    }
    c
}

pub(crate) fn wrap(node: impl IntoNode + 'static) -> NodeBuilder {
    container().child(node)
}

fn save_render(filename: &str, build: impl FnOnce(&mut Cx) -> sigil_kernel::NodeId) {
    save_render_sized(filename, [1100, 800], build);
}

fn save_render_sized(
    filename: &str,
    size: [u32; 2],
    build: impl FnOnce(&mut Cx) -> sigil_kernel::NodeId,
) {
    let img = render_widget_to_png_with(
        build,
        InteractionMode::Default,
        size,
        (1.0, 1.0),
        Box::new(DefaultTextMeasure::new()),
    );
    let Some(img) = img else {
        println!("no GPU adapter — skipping {filename}");
        return;
    };
    let out_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("output");
    fs::create_dir_all(&out_dir).expect("create output dir");
    let path = out_dir.join(filename);
    img.save(&path).expect("save png");
    println!("render → {}", path.display());
}

#[test]
fn render_playground_to_png() {
    save_render("playground.png", |cx| cx.add(root()));
}

#[test]
fn render_playground_full_to_png() {
    save_render_sized("playground_full.png", [1100, 2048], |cx| {
        let content = container()
            .role(Role::Main)
            .label("Widget playground")
            .layout(
                LayoutStyle::flex_column()
                    .padding(PAGE_PADDING)
                    .gap(SECTION_GAP)
                    .fill_width(),
            )
            .class("w-full bg-surface")
            .child(page_header())
            .child(form_section())
            .child(display_section())
            .child(navigation_section())
            .child(container_section())
            .child(reactive_section());
        cx.add(content)
    });
}

#[test]
fn render_playground_dark_to_png() {
    use sigil_style::Theme;
    save_render("playground_dark.png", |cx| {
        cx.add(root().theme(Theme::dark()))
    });
}
