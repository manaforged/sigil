#![allow(dead_code)]

use sigil::geometry::{Point, Rect};
use sigil::style::Color;

pub(crate) const fn color(r: u8, g: u8, b: u8, a: u8) -> Color {
    Color { r, g, b, a }
}

pub(crate) const fn rect(x: f32, y: f32, width: f32, height: f32) -> Rect {
    Rect::new(Point::new(x, y), width, height)
}

/// Renders `scene` once at `view` and `scale` on the GPU, reads it back, and returns the time in milliseconds.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "test views are small and positive"
)]
pub(crate) fn frame_ms(
    renderer: &mut sigil::renderer::WgpuRenderer,
    scene: &sigil::scene::CommandScene,
    view: (f32, f32),
    scale: f32,
) -> f64 {
    use sigil_kittest::gpu::{Capture, read_pixels, render_capture, shared_device};
    let (device, queue) = shared_device().expect("sigil-kittest").expect("adapter");
    let capture =
        Capture::new(view, [(view.0 * scale) as u32, (view.1 * scale) as u32]).scale(scale);
    let start = std::time::Instant::now();
    let texture = render_capture(renderer, (device, queue), scene, &capture).expect("render");
    let [w, h] = capture.size_px();
    std::hint::black_box(read_pixels(device, queue, &texture, w, h).expect("readback"));
    start.elapsed().as_secs_f64() * 1e3
}

/// The median of `values`, which it sorts in place.
pub(crate) fn median(values: &mut [f64]) -> f64 {
    values.sort_by(f64::total_cmp);
    values.get(values.len() / 2).copied().unwrap_or(f64::NAN)
}

/// Renders `scene` on the CPU at `(width, height)` and `scale`, returning an RGBA image.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "test views are small and positive"
)]
pub(crate) fn cpu_frame(
    scene: &sigil::scene::CommandScene,
    (width, height): (u32, u32),
    scale: f32,
    clear: sigil::style::Rgba,
) -> image::RgbaImage {
    let viewport = (width as f32, height as f32);
    let device_w = (viewport.0 * scale).round() as u32;
    let device_h = (viewport.1 * scale).round() as u32;
    let frame = sigil_kittest::parity::Frame::new(
        scene,
        viewport,
        sigil::geometry::PhysicalSize::new(device_w, device_h),
        clear,
    );
    let rgba = sigil_kittest::parity::cpu_rgba(&mut sigil::renderer::CpuRenderer::new(), &frame)
        .expect("cpu render");
    image::RgbaImage::from_raw(device_w, device_h, rgba).expect("cpu frame")
}
