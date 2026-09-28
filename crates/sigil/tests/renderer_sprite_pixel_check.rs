#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]
#![allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
#![allow(clippy::cast_lossless, clippy::cast_sign_loss)]
#![allow(clippy::doc_markdown, clippy::missing_const_for_fn)]
#![allow(clippy::print_stdout, clippy::too_many_lines)]

use sigil::renderer::{OffscreenTarget, WgpuRenderer};
use sigil_kittest::gpu::{FORMAT, create_capture_texture, pixel_rgba};
use sigil_kittest::gpu::{read_pixels, shared_device};

const SCALE: u32 = 1;

const MONO_X: f32 = 80.0;
const MONO_Y: f32 = 80.0;
const SUB_X: f32 = 360.0;
const SUB_Y: f32 = 80.0;
const POLY_X: f32 = 640.0;
const POLY_Y: f32 = 80.0;
const SPRITE_SIDE: f32 = 200.0;

#[test]
#[ignore = "needs a wgpu adapter; run with --ignored on GPU hardware (see README verification)"]
fn sprite_pixels_match_mono_subpixel_poly_contracts() {
    let Some((device, queue)) = shared_device().expect("sigil-kittest").cloned() else {
        println!("no adapter — skipping sprite_pixel_check");
        return;
    };

    let pixels = render_sprite_scene(&device, &queue);
    let phys_w = viewport_phys_w();

    assert_mono_tint_is_red(&pixels, phys_w);
    assert_mono_corner_is_not_red_tint(&pixels, phys_w);
    assert_subpixel_rgb_independence(&pixels, phys_w);
    assert_poly_tint_ignored_green_atlas(&pixels, phys_w);
}

fn viewport_phys_w() -> u32 {
    (900.0_f32 * SCALE as f32) as u32
}

fn render_sprite_scene(device: &wgpu::Device, queue: &wgpu::Queue) -> Vec<u8> {
    let (command_scene, viewport, clear) = sigil::renderer::scenes::sprite();

    let phys_w = (viewport.0 * SCALE as f32) as u32;
    let phys_h = (viewport.1 * SCALE as f32) as u32;

    let texture = create_capture_texture(device, phys_w, phys_h);
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

    WgpuRenderer::new()
        .expect("WgpuRenderer::new")
        .render_command_scene_offscreen(
            OffscreenTarget {
                device,
                queue,
                view: &view,
                format: FORMAT,
                viewport,
                scale: 1.0,
            },
            &command_scene,
            clear,
        )
        .expect("render_command_scene_offscreen");

    read_pixels(device, queue, &texture, phys_w, phys_h).expect("sigil-kittest")
}

fn assert_mono_tint_is_red(pixels: &[u8], phys_w: u32) {
    let (cx, cy) = lp(MONO_X + SPRITE_SIDE / 2.0, MONO_Y + SPRITE_SIDE / 2.0);
    let rgba = pixel_rgba(pixels, phys_w, cx, cy);
    assert!(
        rgba[0] > 180 && rgba[1] < 100 && rgba[2] < 100,
        "Mono tint: center pixel ({cx},{cy}) should be red-tinted \
         (R > 180, G < 100, B < 100). Got R={} G={} B={}. \
         Check sprite_paint.wgsl SPRITE_KIND_MONO branch: \
         coverage * in.color.rgb should amplify R and suppress G/B.",
        rgba[0],
        rgba[1],
        rgba[2],
    );
}

fn assert_mono_corner_is_not_red_tint(pixels: &[u8], phys_w: u32) {
    let (cx, cy) = lp(MONO_X + 5.0, MONO_Y + 5.0);
    let rgba = pixel_rgba(pixels, phys_w, cx, cy);

    let delta_to_tint = color_distance(rgba, [220, 60, 60, 255]);
    let delta_to_black = color_distance(rgba, [0, 0, 0, 255]);

    assert!(
        delta_to_tint > delta_to_black,
        "Mono shape: corner pixel ({cx},{cy}) looks like the tint color rather than \
         the clear color. R={} G={} B={} A={}. \
         dist_to_tint={delta_to_tint} must exceed dist_to_black={delta_to_black}. \
         The synthetic disk should have near-zero alpha at the rect corners.",
        rgba[0],
        rgba[1],
        rgba[2],
        rgba[3],
    );
}

fn assert_subpixel_rgb_independence(pixels: &[u8], phys_w: u32) {
    let (ex, ey) = lp(SUB_X + SPRITE_SIDE - 5.0, SUB_Y + SPRITE_SIDE / 2.0);

    let found = (0..3_u32).any(|dx| {
        let px = pixel_rgba(pixels, phys_w, ex.saturating_sub(dx), ey);
        let r = px[0] as i32;
        let g = px[1] as i32;
        let b = px[2] as i32;
        (r - g).abs() >= 16 || (r - b).abs() >= 16 || (g - b).abs() >= 16
    });

    assert!(
        found,
        "Subpixel RGB independence: scanned three pixels around ({ex},{ey}). \
         None had any channel differing from another channel by ≥ 16. \
         Check sprite_paint.wgsl SPRITE_KIND_SUBPIXEL branch: \
         R/G/B must sample three phase-offset atlas positions independently."
    );
}

fn assert_poly_tint_ignored_green_atlas(pixels: &[u8], phys_w: u32) {
    let (cx, cy) = lp(POLY_X + SPRITE_SIDE / 2.0, POLY_Y + SPRITE_SIDE / 2.0);
    let rgba = pixel_rgba(pixels, phys_w, cx, cy);
    assert!(
        rgba[1] > 180 && rgba[0] < 120,
        "Poly tint-ignored: center pixel ({cx},{cy}) should be bright green \
         from the atlas (G > 180, R < 120). Got R={} G={} B={}. \
         The Poly atlas region is a horizontal red→green gradient; center is \
         ~50% across, so G dominates. Check sprite_paint.wgsl SPRITE_KIND_POLY \
         branch: tint must be dropped, raw atlas RGBA passed through.",
        rgba[0],
        rgba[1],
        rgba[2],
    );
}

fn lp(lx: f32, ly: f32) -> (u32, u32) {
    ((lx * SCALE as f32) as u32, (ly * SCALE as f32) as u32)
}

fn color_distance(a: [u8; 4], b: [u8; 4]) -> u32 {
    (0..4)
        .map(|i| (a[i] as i32 - b[i] as i32).unsigned_abs())
        .sum()
}
