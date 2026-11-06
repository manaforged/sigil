use sigil::prelude::*;
use sigil::scene::{Filter, Shadow};

use sigil::reactive::Owner;
use sigil_kittest::Harness;

use super::{SCALE, SIDE, frames_of, gpu_frame, model, probe, rgba};

const INPUT: Rgba8 = rgba(200, 80, 40, 255);
const BLACK: Rgba8 = rgba(0, 0, 0, 255);
const WHITE: Rgba8 = rgba(255, 255, 255, 255);

fn page(child: impl IntoNode + 'static, pad: f32) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::block()
                .fixed_size(SIDE as f32, SIDE as f32)
                .padding(pad),
        )
        .bg(BLACK)
        .child(child)
}

fn dimmed() -> NodeBuilder {
    page(
        container()
            .layout(LayoutStyle::block().fixed_size(SIDE as f32, SIDE as f32))
            .bg(INPUT)
            .brightness(0.5),
        0.0,
    )
}

#[test]
fn the_brightness_setter_scales_the_node_by_the_spec_matrix() {
    let expected = model::expected_bytes(&[Filter::Brightness(0.5)], INPUT);
    for (name, frame) in frames_of(dimmed) {
        let found = probe(&frame, (30.0, 30.0));
        println!("{name}: brightness(0.5) probe {found:?} model {expected:?}");
        let close = found
            .0
            .iter()
            .zip(expected)
            .all(|(a, b)| a.abs_diff(b) <= 2);
        assert!(
            close,
            "{name}: brightness(0.5) gave {found:?}, the model gives {expected:?}"
        );
    }
}

fn blurred() -> NodeBuilder {
    page(
        container()
            .layout(LayoutStyle::block().fixed_size(20.0, 20.0))
            .bg(WHITE)
            .blur(4.0),
        20.0,
    )
}

#[test]
fn the_blur_setter_spreads_the_node_past_its_box() {
    for (name, frame) in frames_of(blurred) {
        let (outside, centre) = (
            probe(&frame, (44.0, 30.0)).0[0],
            probe(&frame, (30.0, 30.0)).0[0],
        );
        println!("{name}: 4 px outside {outside}, centre {centre}");
        assert!(
            outside > 20,
            "{name}: blur(4) reaches 4 px past the box, got {outside}"
        );
        assert!(
            outside < centre,
            "{name}: the spread fades away from the box, {outside} outside, {centre} centre"
        );
    }
}

fn shadowed_text() -> NodeBuilder {
    page(
        text("HI")
            .font_size(40.0)
            .font_weight(800)
            .text_color(WHITE)
            .text_shadow(Shadow::new([3.0, 3.0], 0.0, 0.0, [1.0, 0.0, 0.0, 1.0])),
        4.0,
    )
}

fn centroid(frame: &image::RgbaImage, pick: impl Fn([u8; 4]) -> bool) -> (f32, f32, f32) {
    let (mut x, mut y, mut n) = (0.0f32, 0.0f32, 0.0f32);
    for (px, py, pixel) in frame.enumerate_pixels() {
        if pick(pixel.0) {
            x += px as f32;
            y += py as f32;
            n += 1.0;
        }
    }
    (x / n.max(1.0), y / n.max(1.0), n)
}

#[test]
fn the_text_shadow_setter_draws_below_and_right_of_the_glyphs() {
    for (name, frame) in frames_of(shadowed_text) {
        let ink = centroid(&frame, |p| p[1] >= 250);
        let shade = centroid(&frame, |p| p[0] >= 200 && p[1] <= 60);
        println!("{name}: ink {ink:?} shadow {shade:?}");
        assert!(
            ink.2 > 50.0 && shade.2 > 50.0,
            "{name}: glyphs and shadow both draw"
        );
        assert!(
            shade.0 > ink.0 && shade.1 > ink.1,
            "{name}: the shadow sits below and right"
        );
    }
}

fn glyphs() -> TextBuilder {
    text("HI").font_size(16.0).font_weight(800)
}

fn stacked(top: impl IntoNode + 'static, parent: fn(NodeBuilder) -> NodeBuilder) -> NodeBuilder {
    parent(
        container()
            .layout(
                LayoutStyle::block()
                    .fixed_size(SIDE as f32, SIDE as f32)
                    .padding(4.0),
            )
            .bg(WHITE)
            .text_color(WHITE),
    )
    .child(
        container()
            .layout(LayoutStyle::block().fixed_size(52.0, 26.0))
            .child(top),
    )
    .child(glyphs())
}

fn darkened(frame: &image::RgbaImage, rows: std::ops::Range<u32>) -> usize {
    frame
        .enumerate_pixels()
        .filter(|(_, y, pixel)| rows.contains(y) && pixel.0[0] < 250)
        .count()
}

fn assert_top_cancelled(name: &str, frame: &image::RgbaImage) {
    let middle = (30.0 * SCALE) as u32;
    let cancelled = darkened(frame, 0..middle);
    let inherited = darkened(frame, middle..frame.height());
    println!("{name}: cancelled child {cancelled}, inheriting sibling {inherited} dark pixels");
    assert_eq!(cancelled, 0, "{name}: an explicit none draws no shadow pixels");
    assert!(inherited > 20, "{name}: the sibling inherits the parent's shadow");
}

#[test]
fn the_text_shadow_none_class_cancels_the_shadow_a_sibling_still_inherits() {
    let app = || {
        stacked(glyphs().class("text-shadow-none"), |parent| {
            parent.class("text-shadow-md")
        })
    };
    for (name, frame) in frames_of(app) {
        assert_top_cancelled(name, &frame);
    }
}

#[test]
fn the_text_shadow_setter_with_none_cancels_the_shadow_a_sibling_still_inherits() {
    let app = || {
        stacked(glyphs().text_shadow(None), |parent| {
            parent.text_shadow(Shadow::new([1.0, 1.0], 1.0, 0.0, [0.0, 0.0, 0.0, 0.5]))
        })
    };
    for (name, frame) in frames_of(app) {
        assert_top_cancelled(name, &frame);
    }
}

fn red_pixels(frame: &image::RgbaImage) -> usize {
    frame
        .pixels()
        .filter(|pixel| pixel.0[0] >= 200 && pixel.0[1] <= 60)
        .count()
}

#[test]
fn turning_a_parent_shadow_off_redraws_the_cached_child_text() {
    Owner::scope("filters-text-shadow-cache", || {
        let off = signal(false);
        let red = Shadow::new([3.0, 3.0], 0.0, 0.0, [1.0, 0.0, 0.0, 1.0]);
        let app = move || {
            container()
                .layout(LayoutStyle::block().fixed_size(SIDE as f32, SIDE as f32))
                .bg(BLACK)
                .text_shadow(red)
                .paint_when(off, |state| state.text_shadow(None))
                .child(text("HI").font_size(40.0).font_weight(800).text_color(WHITE))
        };
        let mut ui = Harness::new(app).pixels(SIDE, SIDE).scale(SCALE);
        let on = red_pixels(&gpu_frame(&mut ui));
        off.set(true);
        ui.settle();
        let cancelled = red_pixels(&gpu_frame(&mut ui));
        off.set(false);
        ui.settle();
        let restored = red_pixels(&gpu_frame(&mut ui));
        println!("red shadow pixels: on {on}, cancelled {cancelled}, restored {restored}");
        assert!(on > 50, "the parent's shadow reaches the child");
        assert_eq!(cancelled, 0, "the cached child drops the shadow");
        assert_eq!(restored, on, "the cached child takes the shadow back");
    });
}
