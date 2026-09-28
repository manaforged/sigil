#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]

use sigil::prelude::*;

const BLUE: Rgba8 = Rgba8 {
    r: 90,
    g: 140,
    b: 255,
    a: 255,
};

fn main() -> sigil::AppResult<()> {
    sigil::run(("Sigil - content", 480, 720), build)
}

fn text_demo() -> TextBuilder {
    text("Text wraps to the width of its parent.")
}

fn label_demo() -> TextBuilder {
    let count = signal(3_i32);
    text(move || format!("Count: {}", count.get())).nowrap()
}

fn icon_demo() -> IconBuilder {
    icon("★").tone(Tone::Warn).label("Favourite")
}

fn image_demo() -> NodeBuilder {
    let pixels: Vec<u8> = (0..32_u32 * 32)
        .flat_map(|i| [u8::try_from(i % 32 * 8).unwrap_or(u8::MAX), 90, 200, 255])
        .collect();
    image_rgba(32, 32, pixels).map_or_else(
        || container().child(text("Image unavailable").nowrap()),
        |picture| container().child(picture.alt("Gradient")),
    )
}

fn path_demo() -> PathNode {
    path(24, 24)
        .move_to(4.0, 20.0)
        .line_to(12.0, 4.0)
        .line_to(20.0, 20.0)
        .close()
        .fill(BLUE)
        .fit()
}

fn chart_demo() -> ChartBuilder {
    chart([12.0, 28.0, 18.0, 34.0])
        .labels(["Q1", "Q2", "Q3", "Q4"])
        .height(160.0)
}

fn section_label_demo() -> NodeBuilder {
    section_label("Recent files")
}

fn notice_band_demo() -> NodeBuilder {
    notice_band("Changes are saved locally until you sign in.", Tone::Warn)
}

fn empty_state_demo() -> NodeBuilder {
    empty_state("No results", "Try a different search term.")
}

fn hint_bar_demo() -> NodeBuilder {
    hint_bar("Enter open · Esc close", "3 items")
}

fn avatar_demo() -> NodeBuilder {
    avatar("AL")
}

fn divider_horizontal_demo() -> NodeBuilder {
    divider_horizontal()
}

fn divider_vertical_demo() -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_row().gap(cols(1.0)))
        .child(text("Left").nowrap())
        .child(divider_vertical())
        .child(text("Right").nowrap())
}

fn hairline_demo() -> NodeBuilder {
    hairline()
}

fn build(cx: &mut Cx) -> NodeId {
    cx.add(
        scroll_auto(
            container()
                .role(Role::Main)
                .layout(LayoutStyle::flex_column().gap(rows(1.0)).padding(cols(2.0)))
                .child(text_demo())
                .child(label_demo())
                .child(icon_demo())
                .child(image_demo())
                .child(path_demo())
                .child(chart_demo())
                .child(section_label_demo())
                .child(notice_band_demo())
                .child(empty_state_demo())
                .child(hint_bar_demo())
                .child(avatar_demo())
                .child(divider_horizontal_demo())
                .child(divider_vertical_demo())
                .child(hairline_demo()),
        )
        .fill(),
    )
}
