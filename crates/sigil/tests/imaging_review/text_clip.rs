use super::{BLACK, BLUE, GREEN, RED, SURFACES, WHITE, check, off, page, shoot};
use sigil::prelude::*;
use sigil::style::GradientAxis;

fn mean_ink(frame: &image::RgbaImage, columns: std::ops::Range<u32>) -> [f32; 3] {
    let (mut sum, mut count) = ([0.0f32; 3], 0.0f32);
    for y in 0..frame.height() {
        for x in columns.clone() {
            let p = frame.get_pixel(x, y);
            let rgb = [p[0], p[1], p[2]];
            if off(rgb, WHITE) > 300 {
                for (total, channel) in sum.iter_mut().zip(rgb) {
                    *total += f32::from(channel);
                }
                count += 1.0;
            }
        }
    }
    assert!(count > 20.0, "glyph pixels in {columns:?}");
    sum.map(|total| total / count)
}

fn word() -> TextBuilder {
    text("MMMMMM")
        .font_size(40.0, "test fixture")
        .font_weight(900)
        .text_color(BLACK)
}

fn container_clip_text() -> NodeBuilder {
    page(
        240.0,
        60.0,
        WHITE,
        container()
            .layout(LayoutStyle::block().fixed_size(240.0, 60.0))
            .bg_linear(GradientAxis::LeftToRight)
            .gradient_from(RED)
            .gradient_to(BLUE)
            .bg_clip_text()
            .child(word()),
    )
}

#[test]
fn bg_clip_text_on_a_container_paints_its_text_children() {
    let mut failures = Vec::new();
    for surface in SURFACES {
        let frame = shoot(container_clip_text, 240, 60, surface);
        let left = mean_ink(&frame, 0..frame.width() / 5);
        if left[0] <= left[2] + 60.0 {
            failures.push(format!("{surface:?}: background-clip:text clips to descendant text, first letters red, got {left:?}"));
        }
    }
    check(&failures);
}

fn conic_word() -> NodeBuilder {
    page(
        240.0,
        60.0,
        WHITE,
        word()
            .bg_conic(0)
            .gradient_from(RED)
            .gradient_via(GREEN)
            .gradient_to(BLUE)
            .bg_clip_text(),
    )
}

#[test]
fn conic_gradient_text_changes_color_across_the_word() {
    let mut failures = Vec::new();
    for surface in SURFACES {
        let frame = shoot(conic_word, 240, 60, surface);
        let width = frame.width();
        let left = mean_ink(&frame, 0..width / 5);
        let right = mean_ink(&frame, width * 3 / 5..width * 4 / 5);
        if !(right[0] > right[2] + 40.0 && left[2] > left[0] + 40.0) {
            failures.push(format!("{surface:?}: right half red-green, left half green-blue; left {left:?} right {right:?}"));
        }
    }
    check(&failures);
}
