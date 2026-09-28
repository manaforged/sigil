#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::expect_used, clippy::indexing_slicing)]

use sigil::hal::{Color, PhysicalSize, Point, Rect};
use sigil::renderer::CpuRenderer;
use sigil::scene::{CommandScene, SceneBuilder, SceneRectStyle};
use sigil::style::Rgba8;

const RED: Rgba8 = Rgba8 {
    r: 255,
    g: 0,
    b: 0,
    a: 255,
};

fn overlapping_group() -> CommandScene {
    let mut scene = SceneBuilder::new();
    scene.with_layer(0.5, |group| {
        group.draw_rect(
            Rect::new(Point::new(0.0, 0.0), 60.0, 40.0),
            SceneRectStyle::solid(RED),
        );
        group.draw_rect(
            Rect::new(Point::new(40.0, 0.0), 60.0, 40.0),
            SceneRectStyle::solid(RED),
        );
    });
    scene.finish()
}

fn pixel(pixmap: &tiny_skia::Pixmap, x: u32, y: u32) -> [u8; 3] {
    let px = pixmap.pixel(x, y).expect("pixel").demultiply();
    [px.red(), px.green(), px.blue()]
}

#[test]
fn cpu_group_opacity_blends_overlap_once() {
    let pixmap = CpuRenderer::new()
        .render(
            &overlapping_group(),
            Color::rgba(1.0, 1.0, 1.0, 1.0),
            (100.0, 40.0),
            PhysicalSize::new(100, 40),
        )
        .expect("cpu render");
    assert_eq!(pixel(&pixmap, 50, 20), pixel(&pixmap, 20, 20));
}

#[test]
#[ignore = "needs a wgpu adapter"]
fn gpu_group_opacity_blends_overlap_once() {
    let Some((device, queue)) = sigil_kittest::gpu::shared_device().expect("gpu") else {
        return;
    };
    let image = sigil_kittest::gpu::render_scene_to_image(
        device,
        queue,
        &overlapping_group(),
        (100.0, 40.0),
        [100, 40],
    )
    .expect("gpu render");
    let at = |x| {
        let px = image.get_pixel(x, 20).0;
        [px[0], px[1], px[2]]
    };
    assert_eq!(at(50), at(20));
}

fn small_group() -> CommandScene {
    let mut scene = SceneBuilder::new();
    scene.with_layer(0.5, |group| {
        group.draw_rect(
            Rect::new(Point::new(10.0, 10.0), 20.0, 20.0),
            SceneRectStyle::solid(RED),
        );
        group.draw_rect(
            Rect::new(Point::new(20.0, 10.0), 20.0, 20.0),
            SceneRectStyle::solid(RED),
        );
    });
    scene.finish()
}

#[test]
fn a_small_group_allocates_only_its_drawn_area() {
    let mut renderer = CpuRenderer::new();
    renderer
        .render(
            &small_group(),
            Color::rgba(1.0, 1.0, 1.0, 1.0),
            (400.0, 300.0),
            PhysicalSize::new(400, 300),
        )
        .expect("cpu render");
    let stats = renderer.last_frame_stats();
    assert_eq!(stats.offscreen_layers, 1);
    assert!(
        stats.offscreen_pixels <= 32 * 22,
        "{}",
        stats.offscreen_pixels
    );
}

#[test]
#[ignore = "needs a wgpu adapter"]
fn gpu_small_group_allocates_only_its_drawn_area() {
    let Some(mut renderer) = sigil_kittest::gpu::headless_renderer().expect("gpu") else {
        return;
    };
    let device = renderer.device().expect("device").clone();
    let queue = renderer.queue().expect("queue").clone();
    let texture = sigil_kittest::gpu::create_capture_texture(&device, 400, 300);
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    renderer
        .render_command_scene_offscreen(
            sigil::renderer::OffscreenTarget {
                device: &device,
                queue: &queue,
                view: &view,
                format: sigil_kittest::gpu::FORMAT,
                viewport: (400.0, 300.0),
                scale: 1.0,
            },
            &small_group(),
            Color::rgba(1.0, 1.0, 1.0, 1.0),
        )
        .expect("gpu render");
    let stats = renderer.last_frame_stats();
    assert_eq!(stats.offscreen_layers, 1);
    assert!(
        stats.offscreen_pixels <= 32 * 22,
        "{}",
        stats.offscreen_pixels
    );
}
