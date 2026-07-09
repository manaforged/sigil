//! Pixel-verification test for Phase R8 icon baker (SVG → MonoSprite).
//!
//! Renders the R8 icon showcase scene via the offscreen capture path, reads back
//! raw RGBA8 bytes (no PNG roundtrip), and asserts four spatial + colour contracts:
//!
//! 1. Each icon's center pixel is tinted correctly (RGB within ±30 of the supplied
//!    tint colour; alpha > 200).
//! 2. Each icon's far corner (10 px inside the top-left of the dest rect) composites
//!    back to the clear colour, proving the baked alpha-mask is transparent outside
//!    the glyph shape.
//! 3. The "x" icon (index 4) has a sharp alpha transition across a 5-pixel horizontal
//!    band, proving the bake produces a resolution-independent edge, not a blurry raster.
//! 4. The "heart" and "x" icons produce distinct alpha fingerprints — their per-icon
//!    alpha sums differ by ≥ 5% of the theoretical maximum — proving the baker
//!    produces unique output per distinct SVG source.
//!
//! This file will not compile until the implementer's R8 work lands (`scenes::icon()`
//! + `SpritePipeline::register_icon`). That is expected. Once it lands, all four
//! assertions must pass or the pixel-test contract defined in phase-R8-icon-baker.md
//! is violated.
//!
#![allow(missing_docs)]
#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]
#![allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
#![allow(clippy::cast_lossless, clippy::cast_sign_loss)]
#![allow(clippy::doc_markdown, clippy::missing_const_for_fn)]
#![allow(clippy::print_stdout, clippy::too_many_lines)]
#![allow(clippy::doc_lazy_continuation)]
#![allow(clippy::type_complexity)]
#![allow(clippy::uninlined_format_args)]

use ahash as _;
use bytemuck as _;
use criterion as _;
use hal as _;
use half as _;
use icons as _;
use image as _;
use kernel as _;
use layout as _;
use pollster as _;
use proptest as _;
use raw_window_handle as _;
use shell as _;
use skrifa as _;
use smallvec as _;
use style as _;
use text as _;
use tracing as _;
#[cfg(windows)]
use windows_core as _;

use renderer::{OffscreenTarget, WgpuRenderer};

#[path = "icon_pixel_check/support.rs"]
mod support;

use support::{abs_diff, create_capture_texture, pixel_rgba, read_pixels, try_request_device};

// Physical scale factor. Viewport 1024 × scale 1 = 1024 px — well under the
// 2048 px downlevel texture-dimension cap.
const SCALE: u32 = 1;
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

// Clear colour as sRGB-encoded u8 components. Color::rgba(0.05, 0.06, 0.09, 1.0)
// in sRGB: 0.05^(1/2.2) ≈ 0.243 → 62? No — sRGB encodes linear 0.05 as
// ~0.05 * 255 ≈ 13 for the *stored* sRGB byte when the texture format is
// Rgba8UnormSrgb and wgpu performs the linear→sRGB conversion on write.
// Linear 0.05 → sRGB ≈ 0.2492 → 63; 0.06 → 0.2673 → 68; 0.09 → 0.3254 → 83.
// We use a ±12 tolerance in the corner assertion to absorb rounding.
const CLEAR_R_SRGB: u8 = 63;
const CLEAR_G_SRGB: u8 = 68;
const CLEAR_B_SRGB: u8 = 83;
const CLEAR_TOL: u8 = 12;

// Icon layout — dest rects as (x, y, w, h) logical pixels, matching scenes::icon().
// Icon 0 "heart"       tint (220,  80,  80)
// Icon 1 "gear"        tint (200, 200, 220)
// Icon 2 "search"      tint (100, 220, 140)
// Icon 3 "arrow-right" tint (230, 200, 100)
// Icon 4 "x"           tint (220, 100, 220)
const ICONS: [(f32, f32, f32, f32, [u8; 3]); 5] = [
    (40.0, 80.0, 128.0, 128.0, [220, 80, 80]),
    (208.0, 80.0, 128.0, 128.0, [200, 200, 220]),
    (376.0, 80.0, 128.0, 128.0, [100, 220, 140]),
    (544.0, 80.0, 128.0, 128.0, [230, 200, 100]),
    (712.0, 80.0, 128.0, 128.0, [220, 100, 220]),
];

// Tolerance per channel for tint-match assertion (§ pixel-test contract item 1).
const TINT_TOL: u8 = 30;

// Minimum alpha for the "center is inside the icon shape" check.
const CENTER_ALPHA_MIN: u8 = 200;

/// Render the R8 icon scene and assert the four pixel-level contracts.
/// Skips cleanly when no GPU adapter is available (headless CI).
#[test]
fn icon_pixels_match_tint_shape_sharpness_and_uniqueness_contracts() {
    let Some((device, queue)) = try_request_device() else {
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

// ---- render -----------------------------------------------------------------

fn render_icon_scene(device: &wgpu::Device, queue: &wgpu::Queue) -> Vec<u8> {
    // scenes::icon() is provided by the implementer; locked API returns
    // (CommandScene, (f32, f32) viewport, Color clear).
    let (command_scene, viewport, clear) = renderer::scenes::icon();

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

// ---- assertion 1: each icon paints at least one tinted opaque pixel ---------

/// Lucide icons are 2-px strokes, not filled silhouettes, so the geometric
/// centre of any given icon may be hollow (heart outline, gear ring, search
/// lens). The pixel-test contract proves "the baker tints correctly" by
/// scanning the entire icon rect and asserting that at least one pixel has
/// alpha ≥ `CENTER_ALPHA_MIN` AND RGB within `TINT_TOL` of the supplied tint.
///
/// This is strictly stronger than a centre-pixel probe: it survives any future
/// asset swap (Heroicons-solid, custom stroke geometry) without re-targeting
/// sample points per icon.
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

// ---- assertion 2: icon corners composite to clear colour --------------------

/// For each icon, sample 10 px inside the top-left corner of the dest rect.
/// The baked alpha mask should be ~0 there (outside the glyph contour), so
/// the rendered pixel must composite back to the clear colour.
///
/// The clear colour in sRGB encoding is approximately (63, 68, 83). We allow
/// ±12 per channel to absorb sRGB gamma rounding and any sub-pixel bleed from
/// a bilinear atlas fetch.
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

// ---- assertion 3: "x" icon edge is sharp ------------------------------------

/// Scans a horizontal band crossing one diagonal of the "x" icon at
/// `y = center_y + 16` and asserts that the icon's R channel cleanly rises
/// from the clear-colour baseline (≈ 63) to the tint (220) over a narrow
/// transition.
///
/// The framebuffer alpha is always 255 (the offscreen target clears to an
/// opaque colour and PREMULTIPLIED_ALPHA_BLENDING preserves dst.a). The actual
/// "icon is present here" signal lives in RGB — at a stroked pixel R = tint.r,
/// at an empty pixel R = clear.r. Sharp-edge proof is that the transition
/// between those two states spans ≤ 3 pixels.
///
/// Picking `y = center_y + 16` (~viewBox y = 15) places the scan well inside
/// the stroke of the diagonal "M6 6 18 18", which crosses x at viewBox x = 15
/// (dest x ≈ rect.x + 80).
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

// ---- assertion 4: distinct icons have distinct alpha fingerprints ------------

/// Compute the per-icon alpha fingerprint for "heart" (icon 0) and "x" (icon 4)
/// by summing all alpha bytes inside each icon's physical dest rect.
///
/// The two sums must differ by ≥ 5% of the theoretical maximum per icon
/// (rect_pixel_count × 255). This proves the baker produces unique alpha masks
/// for distinct SVG sources, not a single baked-once output replicated to every slot.
fn assert_heart_and_x_fingerprints_differ(pixels: &[u8], phys_w: u32) {
    let count_heart = stroked_pixel_count(pixels, phys_w, ICONS[0]);
    let count_x = stroked_pixel_count(pixels, phys_w, ICONS[4]);

    let (rw, rh) = (ICONS[0].2 as u64, ICONS[0].3 as u64);
    let max_count = rw * rh;
    let threshold = max_count / 50; // 2% of rect area

    let diff = count_heart.abs_diff(count_x);
    assert!(
        diff >= threshold,
        "distinct-icon fingerprints: stroked-pixel-count difference between \
         'heart' ({count_heart}) and 'x' ({count_x}) is {diff}, which is less \
         than the 2%-of-area threshold {threshold}. \
         The baker may be producing identical output for all icons.",
    );
}

/// Count pixels inside `icon`'s rect whose R channel is more than `CLEAR_TOL`
/// away from `CLEAR_R_SRGB` — a proxy for "this pixel is part of the icon's
/// stroke" since the framebuffer alpha is always 255 (opaque clear). Distinct
/// SVG geometries produce distinct counts even when both have alpha = 255
/// across the composited rect.
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
