#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]
#![allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
#![allow(clippy::cast_lossless, clippy::cast_sign_loss)]
#![allow(clippy::doc_markdown, clippy::missing_const_for_fn)]
#![allow(clippy::print_stdout, clippy::too_many_lines)]

use sigil::geometry::{Point, Rect};
use sigil::renderer::{OffscreenTarget, WgpuRenderer};
use sigil::scene::{CornerRadii, PatternKind, SceneBuilder, ScenePatternStyle};
use sigil::style::{Color, Rgba8};
use sigil_kittest::gpu::{FORMAT, create_capture_texture, pixel_rgba, read_pixels, shared_device};

const SCALE: u32 = 1;
const VIEWPORT: (f32, f32) = (1200.0, 400.0);

const SX: f32 = 80.0;
const SY: f32 = 80.0;
const SW: f32 = 480.0;
const SH: f32 = 240.0;

const CX: f32 = 620.0;
const CY: f32 = 80.0;

#[test]
#[ignore = "needs a wgpu adapter; run with --ignored on GPU hardware (see README verification)"]
fn pattern_pixels_match_slash_and_checker_contracts() {
    let Some((device, queue)) = shared_device().expect("sigil-kittest").cloned() else {
        println!("no adapter — skipping pattern_pixel_check");
        return;
    };

    let pixels = render_pattern_scene(&device, &queue);
    let phys_w = (VIEWPORT.0 * SCALE as f32) as u32;

    assert_slash_transitions(&pixels, phys_w);
    assert_slash_directionality(&pixels, phys_w);
    assert_checker_alternation(&pixels, phys_w);
    assert_checker_colour_sanity(&pixels, phys_w);
}

fn build_pattern_scene() -> sigil::scene::CommandScene {
    let slash = PatternKind::Slash {
        color: Rgba8 {
            r: 255,
            g: 255,
            b: 255,
            a: 255,
        },
        period: 40.0,
        line_width: 16.0,
    };
    let checker = PatternKind::Checker {
        color_a: Rgba8 {
            r: 220,
            g: 60,
            b: 60,
            a: 255,
        },
        color_b: Rgba8 {
            r: 245,
            g: 245,
            b: 250,
            a: 255,
        },
        cell: 32.0,
    };
    let mut scene = SceneBuilder::new();
    scene.draw_pattern(
        Rect::new(Point::new(SX, SY), SW, SH),
        ScenePatternStyle::new(slash, CornerRadii::ZERO, 10_000),
    );
    scene.draw_pattern(
        Rect::new(Point::new(CX, CY), 480.0, 240.0),
        ScenePatternStyle::new(checker, CornerRadii::ZERO, 10_000),
    );
    scene.finish()
}

fn render_pattern_scene(device: &wgpu::Device, queue: &wgpu::Queue) -> Vec<u8> {
    let phys_w = (VIEWPORT.0 * SCALE as f32) as u32;
    let phys_h = (VIEWPORT.1 * SCALE as f32) as u32;
    let texture = create_capture_texture(device, phys_w, phys_h);
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let scene = build_pattern_scene();
    WgpuRenderer::new()
        .expect("WgpuRenderer::new")
        .render_command_scene_offscreen(
            OffscreenTarget {
                device,
                queue,
                view: &view,
                format: FORMAT,
                viewport: VIEWPORT,
                scale: 1.0,
            },
            &scene,
            Color::rgba(0.06, 0.07, 0.10, 1.0),
        )
        .expect("render_command_scene_offscreen");
    read_pixels(device, queue, &texture, phys_w, phys_h).expect("sigil-kittest")
}

fn assert_slash_transitions(pixels: &[u8], phys_w: u32) {
    let mid_y_phys = ((SY + SH / 2.0) * SCALE as f32) as u32;
    let band_start = mid_y_phys.saturating_sub(16);
    let band_end = (mid_y_phys + 16).min((VIEWPORT.1 * SCALE as f32) as u32 - 1);
    let x_start = (SX * SCALE as f32) as u32;
    let x_end = ((SX + SW) * SCALE as f32) as u32;

    let mut max_transitions = 0u32;
    for y in band_start..=band_end {
        let mut transitions = 0u32;
        let mut prev_bright = false;
        for x in x_start..x_end {
            let r = pixel_r(pixels, phys_w, x, y);
            if r < 80 && prev_bright {
                transitions += 1;
                prev_bright = false;
            } else if r > 200 {
                prev_bright = true;
            }
        }
        if transitions > max_transitions {
            max_transitions = transitions;
        }
    }
    assert!(
        max_transitions >= 4,
        "slash: expected ≥ 4 dark→bright transitions in horizontal scan at \
         mid-y logical; got {max_transitions}. Period=40 × scale=2 = 80 px/cycle \
         across 960 px gives ~12 cycles. Check fill_paint.wgsl fill_kind=1 branch."
    );
}

fn assert_slash_directionality(pixels: &[u8], phys_w: u32) {
    let (ax, ay) = lp(SX + 20.0, SY + 20.0);
    let (bx, by) = lp(SX + 60.0, SY + 60.0);
    let (cx, cy) = lp(SX + 40.0, SY + 20.0);

    let ra = pixel_r(pixels, phys_w, ax, ay) as i32;
    let rb = pixel_r(pixels, phys_w, bx, by) as i32;
    let rc = pixel_r(pixels, phys_w, cx, cy) as i32;

    assert!(
        (ra - rb).abs() < 30,
        "slash 45°: A({ax},{ay}) R={ra} and B({bx},{by}) R={rb} share the same \
         diagonal — diff {} should be < 30. Horizontal stripes would fail here.",
        (ra - rb).abs()
    );
    assert!(
        (ra - rc).abs() > 60,
        "slash 45°: C({cx},{cy}) R={rc} is half-period offset from A R={ra} \
         (opposite stripe phase) — diff {} must be > 60. \
         Constant solid/blank render or wrong slope would fail here.",
        (ra - rc).abs()
    );
}

fn assert_checker_alternation(pixels: &[u8], phys_w: u32) {
    let [a, b, c, d] = four_checker_samples(pixels, phys_w);
    let pat = [is_reddish(a), is_reddish(b), is_reddish(c), is_reddish(d)];
    assert!(
        pat == [true, false, false, true] || pat == [false, true, true, false],
        "checker 2×2: expected alternating reddish/whitish; got {pat:?}. \
         A={a:?} B={b:?} C={c:?} D={d:?}."
    );
}

fn assert_checker_colour_sanity(pixels: &[u8], phys_w: u32) {
    let samples = four_checker_samples(pixels, phys_w);
    assert!(
        samples.iter().any(|&s| is_reddish_tight(s)),
        "checker: no reddish cell matching color_a rgba(220,60,60) within \
         sRGB bounds R∈[200,240], G∈[40,80], B∈[40,80]. Samples: {samples:?}"
    );
    assert!(
        samples.iter().any(|&s| is_whitish(s)),
        "checker: no whitish cell matching color_b rgba(245,245,250) \
         (R,G,B all > 200). Samples: {samples:?}"
    );
}

fn lp(lx: f32, ly: f32) -> (u32, u32) {
    ((lx * SCALE as f32) as u32, (ly * SCALE as f32) as u32)
}

fn four_checker_samples(pixels: &[u8], phys_w: u32) -> [[u8; 4]; 4] {
    let (ax, ay) = lp(CX + 16.0, CY + 16.0);
    let (bx, by) = lp(CX + 48.0, CY + 16.0);
    let (cx, cy) = lp(CX + 16.0, CY + 48.0);
    let (dx, dy) = lp(CX + 48.0, CY + 48.0);
    [
        pixel_rgba(pixels, phys_w, ax, ay),
        pixel_rgba(pixels, phys_w, bx, by),
        pixel_rgba(pixels, phys_w, cx, cy),
        pixel_rgba(pixels, phys_w, dx, dy),
    ]
}

fn pixel_r(pixels: &[u8], width: u32, x: u32, y: u32) -> u8 {
    pixel_rgba(pixels, width, x, y)[0]
}

const fn is_reddish(rgba: [u8; 4]) -> bool {
    rgba[0] > 180 && rgba[1] < 120
}

const fn is_reddish_tight(rgba: [u8; 4]) -> bool {
    rgba[0] >= 200
        && rgba[0] <= 240
        && rgba[1] >= 40
        && rgba[1] <= 80
        && rgba[2] >= 40
        && rgba[2] <= 80
}

const fn is_whitish(rgba: [u8; 4]) -> bool {
    rgba[0] > 200 && rgba[1] > 200 && rgba[2] > 200
}
