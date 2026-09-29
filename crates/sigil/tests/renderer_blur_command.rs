#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used, clippy::panic)]

#[path = "scene_support/mod.rs"]
mod scene_support;

use scene_support::rect;
use sigil::scene::{CommandScene, CornerRadii, LayerEffect, LayerScope, SceneBuilder};
use sigil::style::Color;
use sigil_kittest::gpu::{Capture, render_capture};

const VIEWPORT: (f32, f32) = (128.0, 64.0);
const VIEWPORT_WIDTH: u32 = 128;
const VIEWPORT_HEIGHT: u32 = 64;
const CAPTURE: Capture = Capture {
    viewport: VIEWPORT,
    size_px: [VIEWPORT_WIDTH, VIEWPORT_HEIGHT],
    scale: 1.0,
    clear: Color::rgba(0.05, 0.06, 0.09, 1.0),
};

#[test]
#[ignore = "needs a wgpu adapter; run with --ignored on GPU hardware (see README verification)"]
fn blur_sigma_zero_is_identity() {
    let scene = blur_scene(0.0);
    assert_blur_command_shape(&scene, 0.0);

    let Some(mut renderer) = sigil_kittest::gpu::headless_renderer().expect("sigil-kittest") else {
        return;
    };

    let device = renderer.device().expect("device").clone();
    let queue = renderer.queue().expect("queue").clone();
    render_capture(&mut renderer, (&device, &queue), &scene, &CAPTURE).expect("render_capture");
}

#[test]
#[ignore = "needs a wgpu adapter; run with --ignored on GPU hardware (see README verification)"]
fn blur_sigma_nonzero_renders_without_panic() {
    let scene = blur_scene(6.0);
    assert_blur_command_shape(&scene, 6.0);

    let Some(mut renderer) = sigil_kittest::gpu::headless_renderer().expect("sigil-kittest") else {
        return;
    };

    let device = renderer.device().expect("device").clone();
    let queue = renderer.queue().expect("queue").clone();
    render_capture(&mut renderer, (&device, &queue), &scene, &CAPTURE).expect("render_capture");
}

#[test]
#[ignore = "needs a wgpu adapter; run with --ignored on GPU hardware (see README verification)"]
fn blur_sigma_strong_renders_without_panic() {
    let scene = blur_scene(18.0);

    let Some(mut renderer) = sigil_kittest::gpu::headless_renderer().expect("sigil-kittest") else {
        return;
    };

    let device = renderer.device().expect("device").clone();
    let queue = renderer.queue().expect("queue").clone();
    render_capture(&mut renderer, (&device, &queue), &scene, &CAPTURE).expect("render_capture");
}

fn assert_blur_command_shape(scene: &CommandScene, expected_sigma: f32) {
    let [
        sigil::scene::SceneCommand::PushLayer(scope),
        sigil::scene::SceneCommand::PopLayer,
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
