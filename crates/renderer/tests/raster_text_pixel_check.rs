//! Raster glyph-atlas draw-path check — Wave-3 D2 (DR-055).
//!
//! Renders a small (12px) string through the real command-scene draw path
//! (`shape_glyph_run` → `SceneBuilder` → `WgpuRenderer::render_command_scene_offscreen`)
//! twice: once with [`TextTuning::raster`] on (the new hinted atlas path)
//! and once with it off (today's default — vector Slug). Both must produce
//! ink, and the ink must land in roughly the same region — proving the
//! raster path is actually wired into `draw_glyph_run`'s selection logic,
//! not just unit-tested in isolation.
#![allow(missing_docs)]
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
use sigil_icons as _;
use sigil_kernel as _;
use sigil_layout as _;
use sigil_shell as _;
use sigil_text as _;
use skrifa as _;
use smallvec as _;
use tracing as _;
#[cfg(windows)]
use windows_core as _;
use zeno as _;

use std::sync::mpsc;

use sigil_renderer::{
    CommandScene, OffscreenTarget, SceneBuilder, TextTuning, WgpuRenderer, set_text_tuning,
    shape_glyph_run,
};
use sigil_style::{Rgba8, TextLineHeight, TextStyle};

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const BYTES_PER_PIXEL: u32 = 4;

const LOGICAL_W: f32 = 120.0;
const LOGICAL_H: f32 = 48.0;
const FONT_SIZE: f32 = 12.0;
const ORIGIN: (f32, f32) = (10.0, 8.0);
const TEXT: &str = "Hi there";

/// Renders through the raster path (on) and the vector path (off) and
/// asserts both produce ink whose bounding boxes substantially overlap.
#[test]
fn raster_path_renders_ink_matching_vector_position() {
    let Some(mut renderer) = try_headless_renderer() else {
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

    // Positional sanity, generous tolerance: hinted-raster and analytic-vector
    // AA differ pixel-for-pixel, but both lay out the same glyph run from the
    // same origin — their ink bounding boxes must substantially overlap.
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

/// Render [`TEXT`] once with `TextTuning.raster` set to `raster`, restoring
/// the default tuning afterward so this test stays hermetic within its
/// binary (other `#[test]`s in this file run `--test-threads=1`).
fn render(renderer: &mut WgpuRenderer, raster: bool) -> (Vec<u8>, u32, u32) {
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
            sigil_hal::Color::BLACK,
        )
        .expect("render_command_scene_offscreen");

    set_text_tuning(TextTuning::default());
    let pixels = read_pixels(&device, &queue, &texture, phys_w, phys_h);
    (pixels, phys_w, phys_h)
}

/// Build a `CommandScene` the way an embedding host emits text: one glyph
/// run positioned at [`ORIGIN`], shaped via `shape_glyph_run`.
fn build_text_scene() -> CommandScene {
    let style = TextStyle {
        font_family: String::from("system-ui"),
        font_size: FONT_SIZE,
        font_weight: 400,
        line_height: TextLineHeight::FontSizeRelative(1.2),
    };
    let rect = sigil_hal::Rect::new(
        sigil_hal::Point::new(ORIGIN.0, ORIGIN.1),
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

// ---- measurement --------------------------------------------------------

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

/// Luminance threshold (of 255) above which a pixel counts as "ink".
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

/// Intersection-over-union of two ink bounding boxes.
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

// ---- wgpu plumbing --------------------------------------------------------

/// Adapter-skip guard: build a headless `WgpuRenderer`, or `None` if this
/// machine has no adapter (CI runners without a GPU).
fn try_headless_renderer() -> Option<WgpuRenderer> {
    let mut renderer = WgpuRenderer::new().ok()?;
    renderer.ensure_headless_for_tests().ok()?;
    Some(renderer)
}

fn create_capture_texture(device: &wgpu::Device, width: u32, height: u32) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("raster-text-pixel-check capture"),
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

fn read_pixels(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    width: u32,
    height: u32,
) -> Vec<u8> {
    let unpadded_bpr = width * BYTES_PER_PIXEL;
    let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let padded_bpr = unpadded_bpr.div_ceil(align) * align;
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("raster-text-pixel-check readback"),
        size: u64::from(padded_bpr) * u64::from(height),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("raster-text-pixel-check copy encoder"),
    });
    enc.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded_bpr),
                rows_per_image: Some(height),
            },
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    queue.submit(core::iter::once(enc.finish()));

    let slice = buffer.slice(..);
    let (tx, rx) = mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |r| {
        let _ = tx.send(r);
    });
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("device poll");
    rx.recv().expect("map callback").expect("map result");

    let mapped = slice.get_mapped_range();
    let un = unpadded_bpr as usize;
    let pad = padded_bpr as usize;
    let h = height as usize;
    let mut out = vec![0u8; un * h];
    for row in 0..h {
        out[row * un..row * un + un].copy_from_slice(&mapped[row * pad..row * pad + un]);
    }
    drop(mapped);
    buffer.unmap();
    out
}
