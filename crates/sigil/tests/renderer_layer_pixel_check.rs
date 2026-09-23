#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(missing_docs)]
#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]
#![allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
#![allow(clippy::cast_lossless, clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap, clippy::too_many_arguments, dead_code)]
#![allow(clippy::doc_markdown, clippy::missing_const_for_fn)]
#![allow(clippy::print_stdout, clippy::too_many_lines)]
#![allow(clippy::many_single_char_names, clippy::similar_names)]

use sigil::renderer::{OffscreenTarget, WgpuRenderer};
use sigil_kittest::gpu::{read_pixels, request_device};

const SCALE: u32 = 1;
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const BYTES_PER_PIXEL: u32 = 4;
const VIEWPORT: (f32, f32) = (960.0, 320.0);

const LEFT_RECT: (f32, f32, f32, f32) = (100.0, 80.0, 180.0, 160.0);
const MID_RECT: (f32, f32, f32, f32) = (390.0, 80.0, 180.0, 160.0);

const BLEND_EXPECT_R: u8 = 204;
const BLEND_EXPECT_G: u8 = 207;
const BLEND_EXPECT_B: u8 = 209;
const BLEND_TOL: u8 = 20;

#[test]
#[ignore = "needs a wgpu adapter; run with --ignored on GPU hardware (see README verification)"]
fn layer_pixel_check() {
    let Some((device, queue)) = request_device() else {
        println!("no adapter — skipping layer_pixel_check");
        return;
    };

    let pixels = render_layer_pool_scene(&device, &queue);
    let phys_w = (VIEWPORT.0 * SCALE as f32) as u32;

    assert_layer_free_content_unchanged(&pixels, phys_w);
    assert_layered_middle_rect_at_half_opacity(&pixels, phys_w);
    assert_unlayered_rects_full_opacity(&pixels, phys_w);
    assert_layer_boundary_sharp(&pixels, phys_w);
}

fn render_layer_pool_scene(device: &wgpu::Device, queue: &wgpu::Queue) -> Vec<u8> {
    let (command_scene, viewport, clear) = sigil::renderer::scenes::layer_pool();

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

    read_pixels(device, queue, &texture, phys_w, phys_h)
}

fn assert_layer_free_content_unchanged(pixels: &[u8], phys_w: u32) {
    let x = 50_u32;
    let y = 160_u32;
    let [r, _g, b, _a] = pixel_rgba(pixels, phys_w, x, y);

    assert!(
        r > 80,
        "layer_free_content: at ({x},{y}) R={r}; expected > 80. \
         The two-phase walker may be clearing the main target during Phase 1 \
         snapshot allocation. Check that Phase 1 only reads the command list.",
    );
    assert!(
        b < 80,
        "layer_free_content: at ({x},{y}) B={b}; expected < 80 (red end, not cyan). \
         If B is high, the gradient endpoints may be swapped or the offscreen \
         composite is bleeding into this region.",
    );
}

fn assert_layered_middle_rect_at_half_opacity(pixels: &[u8], phys_w: u32) {
    let cx = (MID_RECT.0 + MID_RECT.2 / 2.0) as u32;
    let cy = (MID_RECT.1 + MID_RECT.3 / 2.0) as u32;
    let [r, g, b, _a] = pixel_rgba(pixels, phys_w, cx, cy);

    assert!(
        abs_diff(r, BLEND_EXPECT_R) <= BLEND_TOL,
        "half_opacity: R at ({cx},{cy})={r}; expected {BLEND_EXPECT_R} ±{BLEND_TOL}. \
         0.5×fill_r(240)+0.5×bg_r(≈174)≈{BLEND_EXPECT_R}. \
         If R≈240 the layer opacity is not applied (check LayerScope::opacity=0.5).",
    );
    assert!(
        abs_diff(g, BLEND_EXPECT_G) <= BLEND_TOL,
        "half_opacity: G at ({cx},{cy})={g}; expected {BLEND_EXPECT_G} ±{BLEND_TOL}. \
         0.5×fill_g(240)+0.5×bg_g(≈174)={BLEND_EXPECT_G}. \
         If G≈240 the composite quad is not modulating by opacity.",
    );
    assert!(
        abs_diff(b, BLEND_EXPECT_B) <= BLEND_TOL,
        "half_opacity: B at ({cx},{cy})={b}; expected {BLEND_EXPECT_B} ±{BLEND_TOL}. \
         0.5×fill_b(250)+0.5×bg_b(≈192)={BLEND_EXPECT_B}. \
         Check that the offscreen texture clears to transparent, not to bg colour.",
    );
}

fn assert_unlayered_rects_full_opacity(pixels: &[u8], phys_w: u32) {
    let cx = (LEFT_RECT.0 + LEFT_RECT.2 / 2.0) as u32;
    let cy = (LEFT_RECT.1 + LEFT_RECT.3 / 2.0) as u32;
    let [r, g, b, _a] = pixel_rgba(pixels, phys_w, cx, cy);

    assert!(
        r > 220,
        "full_opacity: R at ({cx},{cy})={r}; expected > 220. \
         Left rect fill R=240. If R≤220 the walker applies opacity outside the layer clip.",
    );
    assert!(
        g > 220,
        "full_opacity: G at ({cx},{cy})={g}; expected > 220. \
         Left rect fill G=240. Check PopLayer restores the main render target.",
    );
    assert!(
        b > 220,
        "full_opacity: B at ({cx},{cy})={b}; expected > 220. \
         Left rect fill B=250. If B is low, the composite blit may overflow the clip rect.",
    );
}

fn assert_layer_boundary_sharp(pixels: &[u8], phys_w: u32) {
    let y = 160_u32;
    let x_outside = 388_u32;
    let x_inside = 392_u32;

    let [ro, go, _bo, _ao] = pixel_rgba(pixels, phys_w, x_outside, y);
    let [ri, gi, _bi, _ai] = pixel_rgba(pixels, phys_w, x_inside, y);

    let delta_r = abs_diff(ro, ri);
    let delta_g = abs_diff(go, gi);

    assert!(
        delta_r > 30 || delta_g > 30,
        "layer_boundary: no sharp discontinuity at Identity layer left edge (x=390). \
         Outside ({x_outside},{y}): R={ro} G={go}. Inside ({x_inside},{y}): R={ri} G={gi}. \
         ΔR={delta_r}, ΔG={delta_g}; expected at least one > 30. \
         Check that LayerScope::clip is used as the scissor rect for the composite quad.",
    );
}

fn pixel_rgba(pixels: &[u8], width: u32, x: u32, y: u32) -> [u8; 4] {
    let base = ((y * width + x) * BYTES_PER_PIXEL) as usize;
    [
        pixels[base],
        pixels[base + 1],
        pixels[base + 2],
        pixels[base + 3],
    ]
}

fn abs_diff(a: u8, b: u8) -> u8 {
    a.abs_diff(b)
}

fn create_capture_texture(device: &wgpu::Device, width: u32, height: u32) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("layer-pixel-check capture"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    })
}
