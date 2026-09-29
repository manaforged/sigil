#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::cast_lossless)]
#![allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
#![allow(clippy::print_stdout, clippy::doc_markdown, clippy::too_many_lines)]

use sigil::layout::Position;
use sigil::prelude::*;

use sigil_kittest::{InteractionMode, render_widget_to_png_stub};

#[test]
fn position_relative_child_paints() {
    const ROOT_PADDING: u32 = 24;
    const SAMPLE_X: u32 = 64;
    const SAMPLE_Y: u32 = 64;

    let Some(img) = render_widget_to_png_stub(
        || {
            let inner = container()
                .layout(
                    LayoutStyle::block()
                        .fixed_size(40.0, 40.0)
                        .position(Position::Absolute)
                        .inset_left(20.0)
                        .inset_top(20.0),
                )
                .class("bg-accent");
            let middle = container()
                .layout(
                    LayoutStyle::block()
                        .fixed_size(80.0, 80.0)
                        .position(Position::Relative),
                )
                .class("bg-surface-pressed")
                .child(inner);
            container()
                .layout(
                    LayoutStyle::block()
                        .fixed_size(150.0, 150.0)
                        .padding(ROOT_PADDING as f32),
                )
                .class("bg-surface")
                .child(middle)
        },
        InteractionMode::Default,
        [150, 150],
        (0.0, 0.0),
    )
    .expect("sigil-kittest") else {
        println!("no GPU adapter — skipping position_relative_child_paints");
        return;
    };

    let px = img.get_pixel(SAMPLE_X, SAMPLE_Y);
    let [r, g, b, _a] = px.0;
    println!("position_relative_child_paints @ ({SAMPLE_X},{SAMPLE_Y}) = ({r},{g},{b})");

    let close_to_accent =
        (r as i32 - 61).abs() <= 6 && (g as i32 - 107).abs() <= 6 && (b as i32 - 179).abs() <= 6;
    assert!(
        close_to_accent,
        "expected accent ~(61,107,179) at ({SAMPLE_X},{SAMPLE_Y}), got ({r},{g},{b}). \
         Bug A: deeply-nested child paint mislocated (likely scene-emitter \
         double-accumulating parent_origin onto a root-coord layout rect)."
    );
}

#[test]
fn narrow_text_renders_glyph() {
    const ROOT_PADDING: u32 = 24;
    const WIN_X: u32 = 24;
    const WIN_Y: u32 = 24;
    const WIN_W: u32 = 14;
    const WIN_H: u32 = 16;

    let Some(img) = render_widget_to_png_stub(
        || {
            let glyph = text("v")
                .layout(LayoutStyle::block().width(WIN_W as f32))
                .class("text-on-surface");
            let wrapper = container()
                .layout(LayoutStyle::block().fixed_size(16.0, 16.0))
                .class("rounded-sm bg-surface")
                .child(glyph);
            container()
                .layout(
                    LayoutStyle::block()
                        .fixed_size(200.0, 80.0)
                        .padding(ROOT_PADDING as f32),
                )
                .class("bg-surface")
                .child(wrapper)
        },
        InteractionMode::Default,
        [200, 80],
        (0.0, 0.0),
    )
    .expect("sigil-kittest") else {
        println!("no GPU adapter — skipping narrow_text_renders_glyph");
        return;
    };

    let mut dark = 0u32;
    for y in WIN_Y..(WIN_Y + WIN_H) {
        for x in WIN_X..(WIN_X + WIN_W) {
            let px = img.get_pixel(x, y);
            let [r, g, b, _a] = px.0;
            let sum = r as u32 + g as u32 + b as u32;
            if sum < 600 {
                dark += 1;
            }
        }
    }
    println!(
        "narrow_text_renders_glyph: {dark} dark pixels in wrapper window \
         ({WIN_X},{WIN_Y})+{WIN_W}x{WIN_H}"
    );
    assert!(
        dark >= 4,
        "expected ≥4 glyph pixels in wrapper window ({WIN_X},{WIN_Y}) {WIN_W}x{WIN_H}, \
         got {dark}. Bug B: narrow text in nested wrapper failed to rasterise \
         at the expected screen location (likely the same double-count bug as Bug A)."
    );
}
