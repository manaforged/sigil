#![allow(clippy::cast_lossless, clippy::cast_possible_truncation)]
#![allow(clippy::cast_precision_loss, clippy::indexing_slicing)]
#![allow(clippy::unwrap_used)]

use sigil::hal::Size;
use sigil::kernel::Cx;
use sigil::layout::LayoutEngine;
use sigil::scene::emit_scene;
use sigil_kittest::FixedMeasure;

pub(super) const TEXT_MEASURE: FixedMeasure = FixedMeasure {
    width: 40.0,
    height: 16.0,
};

pub(super) fn close(r: u8, g: u8, b: u8, expected: sigil::style::Rgba8, tolerance: i32) -> bool {
    (i32::from(r) - i32::from(expected.r)).abs() <= tolerance
        && (i32::from(g) - i32::from(expected.g)).abs() <= tolerance
        && (i32::from(b) - i32::from(expected.b)).abs() <= tolerance
}

#[derive(Clone, Copy)]
pub(super) struct InkWindow {
    pub(super) x: u32,
    pub(super) y: u32,
    pub(super) w: u32,
    pub(super) h: u32,
}

pub(super) fn count_ink(
    img: &image::RgbaImage,
    window: InkWindow,
    background: sigil::style::Rgba8,
) -> u32 {
    let bg_sum = u32::from(background.r) + u32::from(background.g) + u32::from(background.b);
    let threshold = bg_sum.saturating_sub(150);
    let mut ink = 0u32;
    for py in window.y..(window.y + window.h) {
        for px in window.x..(window.x + window.w) {
            let pixel = img.get_pixel(px, py);
            let [channel_r, channel_g, channel_b, _a] = pixel.0;
            let sum = u32::from(channel_r) + u32::from(channel_g) + u32::from(channel_b);
            if sum < threshold {
                ink += 1;
            }
        }
    }
    ink
}

pub(super) fn capture(
    build: impl FnOnce(&mut Cx) -> sigil::kernel::NodeId,
    size_px: [u32; 2],
) -> image::RgbaImage {
    let mut cx = Cx::new_root();
    let root = build(&mut cx);

    let [w, h] = size_px;
    let mut layout = LayoutEngine::with_text_measure(Box::new(TEXT_MEASURE));
    layout
        .compute(&mut cx, root, Size::fixed(w as f32, h as f32))
        .expect("layout compute");

    let scene = emit_scene(&cx, root);
    let (device, queue) = sigil_kittest::gpu::shared_device()
        .expect("sigil-kittest")
        .expect("the child paint repro needs a GPU");
    sigil_kittest::gpu::render_scene_to_image(device, queue, &scene, (w as f32, h as f32), size_px)
        .expect("sigil-kittest")
}
