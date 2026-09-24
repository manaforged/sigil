#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]
#![allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
#![allow(clippy::cast_lossless, clippy::cast_sign_loss)]
#![allow(clippy::doc_markdown, clippy::missing_const_for_fn)]
#![allow(clippy::print_stdout, clippy::too_many_lines)]
#![allow(clippy::doc_lazy_continuation)]
#![allow(clippy::type_complexity)]
#![allow(clippy::uninlined_format_args)]

use sigil::renderer::{OffscreenTarget, WgpuRenderer};

#[path = "renderer_icon_pixel_check/support.rs"]
mod support;

use support::{abs_diff, create_capture_texture, pixel_rgba, read_pixels, try_request_device};

const SCALE: u32 = 1;
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

const CLEAR_R_SRGB: u8 = 63;
const CLEAR_G_SRGB: u8 = 68;
const CLEAR_B_SRGB: u8 = 83;
const CLEAR_TOL: u8 = 12;

const ICONS: [(f32, f32, f32, f32, [u8; 3]); 5] = [
    (40.0, 80.0, 128.0, 128.0, [220, 80, 80]),
    (208.0, 80.0, 128.0, 128.0, [200, 200, 220]),
    (376.0, 80.0, 128.0, 128.0, [100, 220, 140]),
    (544.0, 80.0, 128.0, 128.0, [230, 200, 100]),
    (712.0, 80.0, 128.0, 128.0, [220, 100, 220]),
];

const TINT_TOL: u8 = 30;

const CENTER_ALPHA_MIN: u8 = 200;

#[test]
#[ignore = "needs a wgpu adapter; run with --ignored on GPU hardware (see README verification)"]
fn icon_pixels_match_tint_shape_sharpness_and_uniqueness_contracts() {
    let Some((device, queue)) = try_request_device().expect("sigil-kittest").cloned() else {
        println!("no adapter — skipping icon_pixel_check");
        return;
    };

    let pixels = render_icon_scene(&device, &queue);
    let phys_w = (1024_f32 * SCALE as f32) as u32;

    assert_icon_centers_are_tinted(&pixels, phys_w);
    assert_icon_corners_are_clear_colour(&pixels, phys_w);
    assert_x_icon_edge_is_sharp(&pixels, phys_w);
    assert_heart_and_x_fingerprints_differ(&pixels, phys_w);
}

fn render_icon_scene(device: &wgpu::Device, queue: &wgpu::Queue) -> Vec<u8> {
    let (command_scene, viewport, clear) = sigil::renderer::scenes::icon();

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

fn assert_icon_centers_are_tinted(pixels: &[u8], phys_w: u32) {
    for (i, &(rx, ry, rw, rh, tint)) in ICONS.iter().enumerate() {
        let x0 = rx as u32;
        let y0 = ry as u32;
        let x1 = x0 + rw as u32;
        let y1 = y0 + rh as u32;
        let mut best: Option<(u32, u32, [u8; 4])> = None;
        for y in y0..y1 {
            for x in x0..x1 {
                let rgba = pixel_rgba(pixels, phys_w, x, y);
                if rgba[3] < CENTER_ALPHA_MIN {
                    continue;
                }
                let dr = abs_diff(rgba[0], tint[0]);
                let dg = abs_diff(rgba[1], tint[1]);
                let db = abs_diff(rgba[2], tint[2]);
                if dr <= TINT_TOL && dg <= TINT_TOL && db <= TINT_TOL {
                    best = Some((x, y, rgba));
                    break;
                }
            }
            if best.is_some() {
                break;
            }
        }
        assert!(
            best.is_some(),
            "icon {i}: no pixel in rect ({x0},{y0})..({x1},{y1}) had alpha ≥ \
             {CENTER_ALPHA_MIN} AND RGB within ±{TINT_TOL} of ({},{},{}). \
             The baker either produced an empty alpha mask, mis-packed the \
             tint, or the sprite atlas binding pointed at the wrong slot.",
            tint[0],
            tint[1],
            tint[2],
        );
    }
}

fn assert_icon_corners_are_clear_colour(pixels: &[u8], phys_w: u32) {
    for (i, &(rx, ry, _rw, _rh, _tint)) in ICONS.iter().enumerate() {
        let px = (rx + 10.0) as u32;
        let py = (ry + 10.0) as u32;
        let rgba = pixel_rgba(pixels, phys_w, px, py);

        let dr = abs_diff(rgba[0], CLEAR_R_SRGB);
        let dg = abs_diff(rgba[1], CLEAR_G_SRGB);
        let db = abs_diff(rgba[2], CLEAR_B_SRGB);
        assert!(
            dr <= CLEAR_TOL && dg <= CLEAR_TOL && db <= CLEAR_TOL,
            "icon {i}: corner pixel ({px},{py}) should be near the clear colour \
             sRGB≈({CLEAR_R_SRGB},{CLEAR_G_SRGB},{CLEAR_B_SRGB}) ±{CLEAR_TOL}. \
             Got RGB=({},{},{}). dR={dr} dG={dg} dB={db}. \
             The alpha mask must be ~0 at the top-left corner for all icons. \
             Check IconBaker::bake viewBox normalisation and raster.rs boundary.",
            rgba[0],
            rgba[1],
            rgba[2],
        );
    }
}

fn assert_x_icon_edge_is_sharp(pixels: &[u8], phys_w: u32) {
    let (rx, ry, _rw, rh, tint) = ICONS[4];
    let scan_y = (ry + rh / 2.0 + 16.0) as u32;
    let scan_x_start = (rx + 60.0) as u32;
    let scan_x_end = scan_x_start + 24;

    let reds: Vec<u8> = (scan_x_start..scan_x_end)
        .map(|x| pixel_rgba(pixels, phys_w, x, scan_y)[0])
        .collect();

    let empty = reds
        .iter()
        .filter(|&&r| r < CLEAR_R_SRGB + CLEAR_TOL)
        .count();
    let stroked = reds.iter().filter(|&&r| r > tint[0] - TINT_TOL).count();

    assert!(
        empty >= 1,
        "x-icon edge sharpness: no empty pixel (R < {}) found in scan band \
         x={scan_x_start}..{scan_x_end} y={scan_y}. R values: {reds:?}",
        CLEAR_R_SRGB + CLEAR_TOL,
    );
    assert!(
        stroked >= 1,
        "x-icon edge sharpness: no stroked pixel (R > {}) found in scan band \
         x={scan_x_start}..{scan_x_end} y={scan_y}. R values: {reds:?}",
        tint[0] - TINT_TOL,
    );

    let first_stroked = reds
        .iter()
        .position(|&r| r > tint[0] - TINT_TOL)
        .unwrap_or(reds.len());
    let last_empty_before = reds
        .iter()
        .take(first_stroked)
        .rposition(|&r| r < CLEAR_R_SRGB + CLEAR_TOL)
        .unwrap_or(0);
    let transition_span = first_stroked.saturating_sub(last_empty_before);
    assert!(
        transition_span <= 4,
        "x-icon edge: empty→stroked transition spans {transition_span} pixels \
         (last_empty={last_empty_before}, first_stroked={first_stroked}) — \
         must be ≤ 4 for a sharp vector edge. R values: {reds:?}. \
         Check raster.rs SSAA quality and the box-filter step.",
    );
}

fn assert_heart_and_x_fingerprints_differ(pixels: &[u8], phys_w: u32) {
    let count_heart = stroked_pixel_count(pixels, phys_w, ICONS[0]);
    let count_x = stroked_pixel_count(pixels, phys_w, ICONS[4]);

    let (rw, rh) = (ICONS[0].2 as u64, ICONS[0].3 as u64);
    let max_count = rw * rh;
    let threshold = max_count / 50;

    let diff = count_heart.abs_diff(count_x);
    assert!(
        diff >= threshold,
        "distinct-icon fingerprints: stroked-pixel-count difference between \
         'heart' ({count_heart}) and 'x' ({count_x}) is {diff}, which is less \
         than the 2%-of-area threshold {threshold}. \
         The baker may be producing identical output for all icons.",
    );
}

fn stroked_pixel_count(pixels: &[u8], phys_w: u32, icon: (f32, f32, f32, f32, [u8; 3])) -> u64 {
    let (rx, ry, rw, rh, _) = icon;
    let x0 = rx as u32;
    let y0 = ry as u32;
    let x1 = x0 + rw as u32;
    let y1 = y0 + rh as u32;
    let mut count = 0_u64;
    for y in y0..y1 {
        for x in x0..x1 {
            if abs_diff(pixel_rgba(pixels, phys_w, x, y)[0], CLEAR_R_SRGB) > CLEAR_TOL {
                count += 1;
            }
        }
    }
    count
}
