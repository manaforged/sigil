use sigil::geometry::PhysicalSize;
use sigil::layout::LayoutStyle;
use sigil::prelude::*;
use sigil::reactive::Owner;
use sigil::renderer::CpuRenderer;
use sigil::style::{MaskImage, MaskSize, Rgba};
use sigil_kittest::Harness;
use sigil_kittest::parity::{Frame, cpu_rgba};

use super::{GAP, RED, SCALE, SIDE, alpha_ramp, assert_ramp, coverage, frames, masked};

fn below_gap(node: NodeBuilder) -> impl IntoNode {
    container()
        .layout(LayoutStyle::flex_column().fixed_size(SIDE as f32, SIDE as f32))
        .child(container().layout(LayoutStyle::block().fixed_size(SIDE as f32, GAP)))
        .child(node)
}

fn red_box() -> NodeBuilder {
    container()
        .layout(LayoutStyle::block().fixed_size(SIDE as f32, SIDE as f32 - GAP))
        .background(RED)
}

fn styled_ramp() -> impl IntoNode {
    below_gap(red_box().mask_image(MaskImage::Linear(90)))
}

#[test]
fn a_linear_mask_image_fades_the_node_from_left_to_right() {
    assert_ramp(styled_ramp, Some((0.4, 0.6)));
}

fn split_image() -> ImageSource {
    let side = SIDE * 2;
    let mut pixels = Vec::with_capacity((side * side * 4) as usize);
    for _ in 0..side {
        for x in 0..side {
            let level = if x < side / 2 { 255 } else { 0 };
            pixels.extend_from_slice(&[level, level, level, 255]);
        }
    }
    ImageSource::from_rgba8(side, side, pixels).expect("mask image")
}

fn split(size: MaskSize) -> NodeBuilder {
    red_box()
        .mask_image(MaskImage::Image(split_image()))
        .mask_size(size)
}

fn split_alpha() -> impl IntoNode {
    below_gap(split(MaskSize::Cover))
}

fn split_luminance() -> impl IntoNode {
    below_gap(split(MaskSize::Cover).mask_luminance())
}

fn split_contained() -> impl IntoNode {
    below_gap(
        split(MaskSize::Contain)
            .mask_luminance()
            .mask_position(0.5, 0.5)
            .mask_repeat(false),
    )
}

#[track_caller]
fn assert_coverage<N: IntoNode + 'static>(app: fn() -> N, expected: &[(f32, bool)]) {
    for (name, frame) in frames(app) {
        for (x, kept) in expected {
            let found = coverage(&frame, (*x, 35.0));
            println!("{name}: x {x} coverage {found:.2}");
            let pass = if *kept { found > 0.9 } else { found < 0.1 };
            assert!(
                pass,
                "{name}: at x {x} expected kept={kept}, coverage {found:.2}"
            );
        }
    }
}

#[test]
fn an_opaque_image_mask_keeps_everything_in_alpha_mode() {
    assert_coverage(split_alpha, &[(27.0, true), (33.0, true)]);
}

#[test]
fn a_luminance_mask_image_keeps_only_its_white_half() {
    assert_coverage(
        split_luminance,
        &[(3.0, true), (27.0, true), (33.0, false), (57.0, false)],
    );
}

#[test]
fn a_contained_centered_mask_image_leaves_its_margins_masked() {
    assert_coverage(
        split_contained,
        &[
            (2.0, false),
            (8.0, true),
            (27.0, true),
            (33.0, false),
            (58.0, false),
        ],
    );
}

const SHIFT: f32 = 20.0;

fn shifted(flag: Signal<bool>) -> impl IntoNode {
    let spacer = |width: f32| {
        move || container().layout(LayoutStyle::block().fixed_size(width, SIDE as f32))
    };
    container()
        .layout(LayoutStyle::flex_row().fixed_size(SIDE as f32 + SHIFT, SIDE as f32))
        .child(show(flag, spacer(SHIFT), spacer(0.0)))
        .child(masked(alpha_ramp))
}

fn moved_frames() -> Vec<(&'static str, image::RgbaImage)> {
    Owner::scope("masks-moved-cache", || {
        let flag = signal(false);
        let width = SIDE + SHIFT as u32;
        let mut ui = Harness::new(shifted(flag)).gpu(width, SIDE).scale(SCALE);
        ui.settle();
        drop(ui.expect_image());
        flag.set(true);
        ui.settle();
        let gpu = ui.expect_image();
        let scene = ui.host().emit((width as f32, SIDE as f32));
        let (device_w, device_h) = ((width as f32 * SCALE) as u32, (SIDE as f32 * SCALE) as u32);
        let frame = Frame::new(
            &scene,
            (width as f32, SIDE as f32),
            PhysicalSize::new(device_w, device_h),
            Rgba::new(0.0, 0.0, 0.0, 1.0),
        );
        let rgba = cpu_rgba(&mut CpuRenderer::new(), &frame).expect("cpu render");
        let cpu = image::RgbaImage::from_raw(device_w, device_h, rgba).expect("cpu frame");
        vec![("gpu", gpu), ("cpu", cpu)]
    })
}

#[test]
fn a_cached_masked_node_that_moves_keeps_its_mask_aligned() {
    for (name, frame) in moved_frames() {
        let at = |x: f32| coverage(&frame, (SHIFT + x, 35.0));
        let (left, middle, right) = (at(3.0), at(30.0), at(57.0));
        println!("{name}: moved coverage {left:.2} {middle:.2} {right:.2}");
        assert!(
            left > 0.85,
            "{name}: the moved left edge keeps the content ({left:.2})"
        );
        assert!(
            (0.4..=0.6).contains(&middle),
            "{name}: the moved midpoint shows {middle:.2}"
        );
        assert!(
            right < 0.15,
            "{name}: the moved right edge is masked away ({right:.2})"
        );
    }
}
