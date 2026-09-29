#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]
#![allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::print_stdout, clippy::too_many_lines)]

use ahash as _;
use bytemuck as _;
use criterion as _;
use half as _;
use image as _;
use pollster as _;
use proptest as _;
use raw_window_handle as _;
use skrifa as _;
use smallvec as _;
use softbuffer as _;
use tiny_skia as _;
use tracing as _;
#[cfg(windows)]
use windows_core as _;
use zeno as _;

use sigil::app::{TextTuning, set_text_tuning};
use sigil::renderer::{OffscreenTarget, WgpuRenderer};
use sigil::scene::{CommandScene, SceneBuilder, shape_glyph_run};
use sigil::style::{Rgba8, TextLineHeight, TextStyle};
use sigil_kittest::gpu::{FORMAT, create_capture_texture, read_pixels};

const BYTES_PER_PIXEL: u32 = 4;

const LOGICAL_W: f32 = 120.0;
const LOGICAL_H: f32 = 48.0;
const FONT_SIZE: f32 = 12.0;
const ORIGIN: (f32, f32) = (10.0, 8.0);
const TEXT: &str = "Hi there";

#[test]
fn raster_path_renders_ink_matching_vector_position() {
    let Some(mut renderer) = sigil_kittest::gpu::headless_renderer().expect("sigil-kittest") else {
        println!("no adapter — skipping raster_text_pixel_check");
        return;
    };

    let (raster_px, w, h) = render(&mut renderer, true);
    let raster = measure(&raster_px, w, h);
    assert!(
        raster.has_ink,
        "raster path (TextTuning.raster=true) produced no ink at {FONT_SIZE}px"
    );

    let (vector_px, vw, vh) = render(&mut renderer, false);
    let vector = measure(&vector_px, vw, vh);
    assert!(
        vector.has_ink,
        "vector path (TextTuning.raster=false) produced no ink at {FONT_SIZE}px"
    );

    let overlap = bbox_overlap(&raster.bbox, &vector.bbox);
    println!(
        "raster bbox={:?} vector bbox={:?} overlap={overlap:.3}",
        raster.bbox, vector.bbox
    );
    assert!(
        overlap > 0.3,
        "raster/vector ink regions barely overlap ({overlap:.3}) — raster bbox {:?}, \
         vector bbox {:?}",
        raster.bbox,
        vector.bbox,
    );
}

fn render(renderer: &mut WgpuRenderer, raster: bool) -> (Vec<u8>, u32, u32) {
    let _guard = sigil::renderer::lock_global_text_state();
    set_text_tuning(TextTuning {
        raster,
        ..TextTuning::default()
    });
    let device = renderer.device().expect("headless device").clone();
    let queue = renderer.queue().expect("headless queue").clone();

    let phys_w = LOGICAL_W.round() as u32;
    let phys_h = LOGICAL_H.round() as u32;
    let texture = create_capture_texture(&device, phys_w, phys_h);
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

    let scene = build_text_scene();
    renderer
        .render_command_scene_offscreen(
            OffscreenTarget {
                device: &device,
                queue: &queue,
                view: &view,
                format: FORMAT,
                viewport: (LOGICAL_W, LOGICAL_H),
                scale: 1.0,
            },
            &scene,
            sigil::style::Color::BLACK,
        )
        .expect("render_command_scene_offscreen");

    set_text_tuning(TextTuning::default());
    let pixels = read_pixels(&device, &queue, &texture, phys_w, phys_h).expect("sigil-kittest");
    (pixels, phys_w, phys_h)
}

fn build_text_scene() -> CommandScene {
    let style = TextStyle {
        font_family: String::from("system-ui"),
        font_size: FONT_SIZE,
        font_weight: 400,
        line_height: TextLineHeight::FontSizeRelative(1.2),
        ..TextStyle::default()
    };
    let rect = sigil::geometry::Rect::new(
        sigil::geometry::Point::new(ORIGIN.0, ORIGIN.1),
        LOGICAL_W - ORIGIN.0,
        LOGICAL_H - ORIGIN.1,
    );
    let run = shape_glyph_run(
        rect,
        TEXT.to_string(),
        Rgba8 {
            r: 255,
            g: 255,
            b: 255,
            a: 255,
        },
        style,
        10_000,
    );
    let mut builder = SceneBuilder::new();
    builder.draw_glyph_run(run);
    builder.finish()
}

#[derive(Debug, Clone, Copy)]
struct BBox {
    x0: u32,
    y0: u32,
    x1: u32,
    y1: u32,
}

struct Measurement {
    has_ink: bool,
    bbox: BBox,
}

const INK_THRESHOLD: u8 = 40;

fn measure(pixels: &[u8], width: u32, height: u32) -> Measurement {
    let mut x0 = width;
    let mut y0 = height;
    let mut x1 = 0u32;
    let mut y1 = 0u32;
    let mut has_ink = false;
    for y in 0..height {
        for x in 0..width {
            let idx = ((y * width + x) * BYTES_PER_PIXEL) as usize;
            let Some(&r) = pixels.get(idx) else { continue };
            if r > INK_THRESHOLD {
                has_ink = true;
                x0 = x0.min(x);
                y0 = y0.min(y);
                x1 = x1.max(x + 1);
                y1 = y1.max(y + 1);
            }
        }
    }
    let bbox = if has_ink {
        BBox { x0, y0, x1, y1 }
    } else {
        BBox {
            x0: 0,
            y0: 0,
            x1: 0,
            y1: 0,
        }
    };
    Measurement { has_ink, bbox }
}

fn bbox_overlap(a: &BBox, b: &BBox) -> f32 {
    let ix0 = a.x0.max(b.x0);
    let iy0 = a.y0.max(b.y0);
    let ix1 = a.x1.min(b.x1);
    let iy1 = a.y1.min(b.y1);
    if ix1 <= ix0 || iy1 <= iy0 {
        return 0.0;
    }
    let inter = f32::from(u16::try_from(ix1 - ix0).unwrap_or(u16::MAX))
        * f32::from(u16::try_from(iy1 - iy0).unwrap_or(u16::MAX));
    let area_a = f32::from(u16::try_from(a.x1 - a.x0).unwrap_or(u16::MAX))
        * f32::from(u16::try_from(a.y1 - a.y0).unwrap_or(u16::MAX));
    let area_b = f32::from(u16::try_from(b.x1 - b.x0).unwrap_or(u16::MAX))
        * f32::from(u16::try_from(b.y1 - b.y0).unwrap_or(u16::MAX));
    let union = area_a + area_b - inter;
    if union <= 0.0 { 0.0 } else { inter / union }
}
