#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]

use sigil::kernel::TextBuilder;
use sigil::prelude::*;
use sigil::widgets::{ChartBuilder, IconBuilder};

const BLUE: Rgba8 = Rgba8 {
    r: 90,
    g: 140,
    b: 255,
    a: 255,
};

fn main() -> sigil::AppResult<()> {
    sigil::run(AppConfig::new("Sigil - content").size(480, 720), build)
}

pub(crate) fn text_demo() -> TextBuilder {
    text("Text wraps to the width of its parent and flows onto the next line.")
}

pub(crate) fn label_demo() -> TextBuilder {
    text("Labels and text share TextContent.").nowrap()
}

pub(crate) fn icon_demo() -> IconBuilder {
    icon("▲").tone(Tone::Warn).label("Alert")
}

pub(crate) fn image_rgba_demo() -> NodeBuilder {
    let pixels: Vec<u8> = (0..32_u32 * 32)
        .flat_map(|i| {
            let shade = u8::try_from((i % 32) * 8).unwrap_or(u8::MAX);
            [shade, 120, 255 - shade, 255]
        })
        .collect();
    let frame = container().layout(LayoutStyle::flex_column().fill_width());
    match image_rgba(32, 32, pixels) {
        Some(picture) => frame.child(picture),
        None => frame.child(text("image_rgba rejected").nowrap()),
    }
}

pub(crate) fn path_demo() -> PathNode {
    path(24, 24)
        .move_to(4.0, 20.0)
        .line_to(12.0, 4.0)
        .line_to(20.0, 20.0)
        .close()
        .fill(BLUE)
        .fit()
}

pub(crate) fn chart_demo() -> ChartBuilder {
    chart([12.0, 28.0, 18.0, 34.0])
        .labels(["Q1", "Q2", "Q3", "Q4"])
        .height(160.0)
}

pub(crate) fn section_label_demo() -> NodeBuilder {
    section_label("Section")
}

pub(crate) fn notice_band_demo() -> NodeBuilder {
    notice_band("Deployment finished", Tone::Success)
}

pub(crate) fn empty_state_demo() -> NodeBuilder {
    empty_state("No results", "Try a different search term.")
}

pub(crate) fn hint_bar_demo() -> NodeBuilder {
    hint_bar("↑↓ navigate", "Enter select")
}

pub(crate) fn avatar_demo() -> NodeBuilder {
    avatar("TG")
}

pub(crate) fn divider_horizontal_demo() -> NodeBuilder {
    divider_horizontal()
}

pub(crate) fn divider_vertical_demo() -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_row().height(40.0))
        .child(divider_vertical())
}

fn build() -> impl IntoNode {
    scroll(
        container()
            .role(Role::Main)
            .layout(LayoutStyle::flex_column().gap(rows(1.0)).padding(cols(2.0)))
            .child(text_demo())
            .child(label_demo())
            .child(icon_demo())
            .child(image_rgba_demo())
            .child(path_demo())
            .child(chart_demo())
            .child(section_label_demo())
            .child(notice_band_demo())
            .child(empty_state_demo())
            .child(hint_bar_demo())
            .child(avatar_demo())
            .child(divider_horizontal_demo())
            .child(divider_vertical_demo()),
    )
    .fill()
}
