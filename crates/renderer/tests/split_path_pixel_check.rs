#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(missing_docs)]
#![allow(deprecated)]
#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]
#![allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
#![allow(clippy::cast_lossless, clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap, dead_code)]
#![allow(clippy::doc_markdown, clippy::missing_const_for_fn)]
#![allow(clippy::print_stdout, clippy::too_many_lines)]
#![allow(clippy::many_single_char_names, clippy::similar_names)]

use sigil_hal::Color;
use sigil_renderer::{
    CommandScene, CornerRadii, OffscreenTarget, SceneBuilder, SceneRectStyle, ScreenSubmission,
    WgpuRenderer, WorldSubmission,
};
use sigil_style::Rgba8;

#[path = "split_path_pixel_check/support.rs"]
mod support;

use support::{
    FORMAT, create_capture_texture, create_depth_texture, linear_to_srgb_u8, pixel_rgba,
    pre_clear_texture, read_pixels, try_request_device,
};

const PHYS_W: u32 = 1280;
const PHYS_H: u32 = 400;
const VIEWPORT: (f32, f32) = (1280.0, 400.0);

const RED_RECT_X: f32 = 400.0;
const RED_RECT_Y: f32 = 100.0;
const RED_RECT_W: f32 = 480.0;
const RED_RECT_H: f32 = 200.0;

const fn identity_4x4() -> [[f32; 4]; 4] {
    [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]
}

#[test]
#[ignore = "needs a wgpu adapter; run with --ignored on GPU hardware (see README verification)"]
fn assert_submit_screen_renders_into_offscreen() {
    let Some((device, queue)) = try_request_device("split-path-screen") else {
        println!("no adapter — skipping split_path_pixel_check");
        return;
    };

    let texture = create_capture_texture(&device, PHYS_W, PHYS_H, "screen-target");
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let scene = build_red_rect_scene();

    WgpuRenderer::new()
        .expect("WgpuRenderer::new")
        .submit_screen(
            &ScreenSubmission {
                device: &device,
                queue: &queue,
                view: &view,
                format: FORMAT,
                viewport: VIEWPORT,
                scale: 1.0,
                clear: Some(Color::rgba(0.0, 0.0, 0.0, 1.0)),
            },
            &scene,
        )
        .expect("submit_screen");

    let pixels = read_pixels(&device, &queue, &texture, PHYS_W, PHYS_H);

    assert_pixel_dark(&pixels, 32, 32, "clear region (32,32)");
    let cx = (RED_RECT_X + RED_RECT_W / 2.0) as u32;
    let cy = (RED_RECT_Y + RED_RECT_H / 2.0) as u32;
    assert_pixel_red(&pixels, cx, cy, "red rect centre");
}

#[test]
#[ignore = "needs a wgpu adapter; run with --ignored on GPU hardware (see README verification)"]
fn assert_renderer_reusable_across_targets() {
    let Some((device, queue)) = try_request_device("split-path-reuse") else {
        println!("no adapter — skipping split_path_pixel_check");
        return;
    };

    let (scene_a, vp_a, clear_a) = sigil_renderer::scenes::gradient();
    let (scene_b, vp_b, clear_b) = sigil_renderer::scenes::pattern();

    let tex_a = create_capture_texture(&device, vp_a.0 as u32, vp_a.1 as u32, "reuse-a");
    let view_a = tex_a.create_view(&wgpu::TextureViewDescriptor::default());
    let tex_b = create_capture_texture(&device, vp_b.0 as u32, vp_b.1 as u32, "reuse-b");
    let view_b = tex_b.create_view(&wgpu::TextureViewDescriptor::default());

    let mut renderer = WgpuRenderer::new().expect("WgpuRenderer::new");

    renderer
        .submit_screen(
            &ScreenSubmission {
                device: &device,
                queue: &queue,
                view: &view_a,
                format: FORMAT,
                viewport: vp_a,
                scale: 1.0,
                clear: Some(clear_a),
            },
            &scene_a,
        )
        .expect("submit_screen A");

    renderer
        .submit_screen(
            &ScreenSubmission {
                device: &device,
                queue: &queue,
                view: &view_b,
                format: FORMAT,
                viewport: vp_b,
                scale: 1.0,
                clear: Some(clear_b),
            },
            &scene_b,
        )
        .expect("submit_screen B");

    let px_a = read_pixels(&device, &queue, &tex_a, vp_a.0 as u32, vp_a.1 as u32);
    let px_b = read_pixels(&device, &queue, &tex_b, vp_b.0 as u32, vp_b.1 as u32);

    assert_target_a_gradient_not_corrupted(&px_a, vp_a.0 as u32);
    assert_target_b_pattern_not_corrupted(&px_b, vp_b.0 as u32);
}

#[test]
#[ignore = "needs a wgpu adapter; run with --ignored on GPU hardware (see README verification)"]
fn assert_deprecated_offscreen_forwards() {
    let Some((device, queue)) = try_request_device("split-path-compat") else {
        println!("no adapter — skipping split_path_pixel_check");
        return;
    };

    let scene = build_red_rect_scene();
    let clear = Color::rgba(0.0, 0.0, 0.0, 1.0);

    let tex_dep = create_capture_texture(&device, PHYS_W, PHYS_H, "deprecated-target");
    let view_dep = tex_dep.create_view(&wgpu::TextureViewDescriptor::default());
    let tex_new = create_capture_texture(&device, PHYS_W, PHYS_H, "new-target");
    let view_new = tex_new.create_view(&wgpu::TextureViewDescriptor::default());

    WgpuRenderer::new()
        .expect("deprecated renderer")
        .render_command_scene_offscreen(
            OffscreenTarget {
                device: &device,
                queue: &queue,
                view: &view_dep,
                format: FORMAT,
                viewport: VIEWPORT,
                scale: 1.0,
            },
            &scene,
            clear,
        )
        .expect("render_command_scene_offscreen");

    WgpuRenderer::new()
        .expect("new renderer")
        .submit_screen(
            &ScreenSubmission {
                device: &device,
                queue: &queue,
                view: &view_new,
                format: FORMAT,
                viewport: VIEWPORT,
                scale: 1.0,
                clear: Some(clear),
            },
            &scene,
        )
        .expect("submit_screen");

    let pix_dep = read_pixels(&device, &queue, &tex_dep, PHYS_W, PHYS_H);
    let pix_new = read_pixels(&device, &queue, &tex_new, PHYS_W, PHYS_H);
    assert_bytes_near_identical(&pix_dep, &pix_new, 2);
}

#[test]
#[ignore = "needs a wgpu adapter; run with --ignored on GPU hardware (see README verification)"]
fn assert_submit_world_stub_returns_ok() {
    let Some((device, queue)) = try_request_device("split-path-world") else {
        println!("no adapter — skipping split_path_pixel_check");
        return;
    };

    let scene = SceneBuilder::new().finish();
    let pre_clear = Color::rgba(0.3, 0.0, 0.5, 1.0);

    let tex_world = create_capture_texture(&device, PHYS_W, PHYS_H, "world-target");
    let view_world = tex_world.create_view(&wgpu::TextureViewDescriptor::default());
    pre_clear_texture(&device, &queue, &view_world, pre_clear);

    let depth_tex = create_depth_texture(&device, PHYS_W, PHYS_H);
    let depth_view = depth_tex.create_view(&wgpu::TextureViewDescriptor::default());

    let result = WgpuRenderer::new()
        .expect("WgpuRenderer::new")
        .submit_world(
            &WorldSubmission {
                device: &device,
                queue: &queue,
                view: &view_world,
                format: FORMAT,
                depth: &depth_view,
                view_proj: identity_4x4(),
                viewport: VIEWPORT,
            },
            &scene,
        );
    assert!(result.is_ok(), "submit_world stub returned Err: {result:?}");

    let pixels = read_pixels(&device, &queue, &tex_world, PHYS_W, PHYS_H);
    assert_stub_left_pre_clear_intact(&pixels, pre_clear);
}

fn build_red_rect_scene() -> CommandScene {
    use sigil_hal::{Point, Rect};
    let mut scene = SceneBuilder::new();
    scene.draw_rect(
        Rect::new(Point::new(RED_RECT_X, RED_RECT_Y), RED_RECT_W, RED_RECT_H),
        SceneRectStyle::new(
            Rgba8 {
                r: 220,
                g: 20,
                b: 20,
                a: 255,
            },
            CornerRadii::ZERO,
            10_000,
        ),
    );
    scene.finish()
}

fn assert_pixel_dark(pixels: &[u8], x: u32, y: u32, label: &str) {
    let [r, g, b, _] = pixel_rgba(pixels, PHYS_W, x, y);
    assert!(
        r < 10 && g < 10 && b < 10,
        "{label}: at ({x},{y}) expected dark (R<10,G<10,B<10), got R={r} G={g} B={b}. \
         submit_screen clear may not be applied or scene draws outside the rect.",
    );
}

fn assert_pixel_red(pixels: &[u8], x: u32, y: u32, label: &str) {
    let [r, g, b, _] = pixel_rgba(pixels, PHYS_W, x, y);
    assert!(
        r > 200 && g < 110 && b < 110 && r > g + 100 && r > b + 100,
        "{label}: at ({x},{y}) expected red-dominant (R>200, G<110, B<110, R-G>100), \
         got R={r} G={g} B={b}. \
         submit_screen may not be painting the red rect at ({RED_RECT_X},{RED_RECT_Y}).",
    );
}

fn assert_target_a_gradient_not_corrupted(pixels: &[u8], phys_w: u32) {
    let [r, g, b, _] = pixel_rgba(pixels, phys_w, 10, 10);
    assert!(
        r < 80 && g < 80 && b < 80,
        "target A corrupted: at (10,10) expected dark bg, got R={r} G={g} B={b}. \
         Pipeline state may have bled from the second submit_screen call.",
    );
}

fn assert_target_b_pattern_not_corrupted(pixels: &[u8], phys_w: u32) {
    let [r, g, b, _] = pixel_rgba(pixels, phys_w, 330, 190);
    let luminance = u32::from(r) + u32::from(g) + u32::from(b);
    assert!(
        luminance > 100,
        "target B corrupted: at (330,190) luminance={luminance} (expected >100). \
         Pattern fill may have been replaced by gradient background from target A.",
    );
}

fn assert_bytes_near_identical(deprecated: &[u8], new_api: &[u8], tol: u8) {
    assert_eq!(
        deprecated.len(),
        new_api.len(),
        "pixel buffer lengths differ: deprecated={} new={}",
        deprecated.len(),
        new_api.len(),
    );
    let mismatches: Vec<usize> = deprecated
        .iter()
        .zip(new_api.iter())
        .enumerate()
        .filter_map(
            |(i, (&d, &n))| {
                if d.abs_diff(n) > tol { Some(i) } else { None }
            },
        )
        .collect();
    assert!(
        mismatches.is_empty(),
        "deprecated and new API outputs differ at {} positions (tol ±{tol}). \
         First mismatch: byte[{}] deprecated={} new={}.",
        mismatches.len(),
        mismatches[0],
        deprecated[mismatches[0]],
        new_api[mismatches[0]],
    );
}

fn assert_stub_left_pre_clear_intact(pixels: &[u8], pre_clear: Color) {
    let exp_r = linear_to_srgb_u8(pre_clear.r);
    let exp_g = linear_to_srgb_u8(pre_clear.g);
    let exp_b = linear_to_srgb_u8(pre_clear.b);
    for row in 0..4_u32 {
        for col in 0..4_u32 {
            let x = 50 + col * 300;
            let y = 50 + row * 80;
            let [r, g, b, _] = pixel_rgba(pixels, PHYS_W, x, y);
            assert!(
                r.abs_diff(exp_r) <= 20 && g.abs_diff(exp_g) <= 20 && b.abs_diff(exp_b) <= 20,
                "submit_world stub painted at ({x},{y}). \
                 Expected pre-clear sRGB≈({exp_r},{exp_g},{exp_b}), got ({r},{g},{b}). \
                 The stub must return Ok without issuing any draw commands.",
            );
        }
    }
}
