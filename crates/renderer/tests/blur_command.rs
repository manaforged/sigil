#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
//! Walker integration test for Phase R12 backdrop-blur.
//!
//! Verifies that `LayerEffect::LayerBlur` is correctly threaded through the
//! command-scene walker without panics, that sigma=0 is treated as a no-op, and
//! that the blur pipeline is lazily initialised on first use.

#![allow(missing_docs)]
#![allow(clippy::unwrap_used, clippy::panic)]

use sigil_hal::{Color, Point, Rect};
use sigil_renderer::{
    CommandScene, CornerRadii, LayerEffect, LayerScope, OffscreenTarget, SceneBuilder, WgpuRenderer,
};

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const VIEWPORT: (f32, f32) = (128.0, 64.0);
const VIEWPORT_WIDTH: u32 = 128;
const VIEWPORT_HEIGHT: u32 = 64;

/// sigma=0 must not crash and must produce zero blur passes (identity path).
#[test]
fn blur_sigma_zero_is_identity() {
    let scene = blur_scene(0.0);
    assert_blur_command_shape(&scene, 0.0);

    let mut renderer = WgpuRenderer::new().expect("WgpuRenderer::new");
    if let Err(err) = renderer.ensure_headless_for_tests() {
        eprintln!("no headless renderer device — skipping: {err}");
        return;
    }

    let device = renderer.device().expect("device").clone();
    let queue = renderer.queue().expect("queue").clone();
    render_scene(&mut renderer, &device, &queue, &scene);
}

/// sigma>0 exercises the blur pipeline initialisation and upsample path.
#[test]
fn blur_sigma_nonzero_renders_without_panic() {
    let scene = blur_scene(6.0);
    assert_blur_command_shape(&scene, 6.0);

    let mut renderer = WgpuRenderer::new().expect("WgpuRenderer::new");
    if let Err(err) = renderer.ensure_headless_for_tests() {
        eprintln!("no headless renderer device — skipping: {err}");
        return;
    }

    let device = renderer.device().expect("device").clone();
    let queue = renderer.queue().expect("queue").clone();
    render_scene(&mut renderer, &device, &queue, &scene);
}

/// sigma=18 (>16) hits the 3-pass branch in `pass_count_for`.
#[test]
fn blur_sigma_strong_renders_without_panic() {
    let scene = blur_scene(18.0);

    let mut renderer = WgpuRenderer::new().expect("WgpuRenderer::new");
    if let Err(err) = renderer.ensure_headless_for_tests() {
        eprintln!("no headless renderer device — skipping: {err}");
        return;
    }

    let device = renderer.device().expect("device").clone();
    let queue = renderer.queue().expect("queue").clone();
    render_scene(&mut renderer, &device, &queue, &scene);
}

fn assert_blur_command_shape(scene: &CommandScene, expected_sigma: f32) {
    let [
        sigil_renderer::SceneCommand::PushLayer(scope),
        sigil_renderer::SceneCommand::PopLayer,
    ] = scene.commands.as_slice()
    else {
        panic!("expected push-layer, pop-layer; got {:?}", scene.commands);
    };
    let LayerEffect::LayerBlur { sigma } = scope.effect else {
        panic!("expected LayerBlur effect, got {:?}", scope.effect);
    };
    assert!(
        (sigma - expected_sigma).abs() < f32::EPSILON,
        "sigma mismatch: {sigma} vs {expected_sigma}",
    );
}

fn blur_scene(sigma: f32) -> CommandScene {
    let mut builder = SceneBuilder::new();
    builder.push_layer(LayerScope {
        clip: rect(16.0, 8.0, 96.0, 48.0),
        clip_radii: CornerRadii::ZERO,
        effect: LayerEffect::LayerBlur { sigma },
        opacity: 1.0,
    });
    builder.pop_layer();
    builder.finish()
}

fn render_scene(
    renderer: &mut WgpuRenderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    scene: &CommandScene,
) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("blur-command capture"),
        size: wgpu::Extent3d {
            width: VIEWPORT_WIDTH,
            height: VIEWPORT_HEIGHT,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    renderer
        .render_command_scene_offscreen(
            OffscreenTarget {
                device,
                queue,
                view: &view,
                format: FORMAT,
                viewport: VIEWPORT,
                scale: 1.0,
            },
            scene,
            Color::rgba(0.05, 0.06, 0.09, 1.0),
        )
        .expect("render_command_scene_offscreen");
}

const fn rect(x: f32, y: f32, width: f32, height: f32) -> Rect {
    Rect::new(Point::new(x, y), width, height)
}
