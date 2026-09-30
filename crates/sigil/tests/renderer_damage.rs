#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used, clippy::indexing_slicing)]
#![allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]

use sigil::geometry::{Point, Rect};
use sigil::renderer::{ScreenSubmission, WgpuRenderer};
use sigil::scene::{CommandScene, CornerRadii, SceneBuilder, SceneRectStyle};
use sigil::style::{Color, Rgba8};
use sigil_kittest::gpu::read_pixels;

const W: u32 = 128;
const H: u32 = 128;
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const CLEAR: Color = Color {
    r: 0.02,
    g: 0.03,
    b: 0.05,
    a: 1.0,
};

fn square(at: (f32, f32), color: (u8, u8, u8)) -> CommandScene {
    let mut b = SceneBuilder::new();
    b.draw_rect(
        Rect::new(Point::new(64.0, 0.0), 64.0, H as f32),
        style(40, 40, 60),
    );
    b.draw_rect(
        Rect::new(Point::new(at.0, at.1), 16.0, 16.0),
        style(color.0, color.1, color.2),
    );
    b.finish()
}

const fn style(r: u8, g: u8, b: u8) -> SceneRectStyle {
    SceneRectStyle::new(Rgba8 { r, g, b, a: 255 }, CornerRadii::ZERO, 10_000)
}

fn render(renderer: &mut WgpuRenderer, texture: &wgpu::Texture, scenes: &[CommandScene]) -> u64 {
    let (device, queue) = sigil_kittest::gpu::shared_device()
        .expect("sigil-kittest")
        .cloned()
        .expect("gpu");
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let target = ScreenSubmission {
        device: &device,
        queue: &queue,
        view: &view,
        format: FORMAT,
        viewport: (W as f32, H as f32),
        scale: 1.0,
        clear: Some(CLEAR),
    };
    for scene in scenes {
        renderer.submit_screen(&target, scene).unwrap();
    }
    renderer.last_frame_stats().redrawn_pixels
}

fn texture() -> wgpu::Texture {
    let (device, _) = sigil_kittest::gpu::shared_device()
        .expect("sigil-kittest")
        .cloned()
        .expect("gpu");
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("damage test target"),
        size: wgpu::Extent3d {
            width: W,
            height: H,
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

fn pixels(texture: &wgpu::Texture) -> Vec<u8> {
    let (device, queue) = sigil_kittest::gpu::shared_device()
        .expect("sigil-kittest")
        .cloned()
        .expect("gpu");
    read_pixels(&device, &queue, texture, W, H).expect("sigil-kittest")
}

#[test]
#[ignore = "needs a wgpu adapter; run with --ignored on GPU hardware (see README verification)"]
fn a_moved_square_redraws_only_its_region_and_matches_a_full_frame() {
    if sigil_kittest::gpu::shared_device()
        .expect("sigil-kittest")
        .is_none()
    {
        return;
    }
    let before = square((8.0, 8.0), (220, 30, 30));
    let after = square((88.0, 40.0), (30, 200, 60));

    let partial = texture();
    let redrawn = render(
        &mut WgpuRenderer::new().unwrap(),
        &partial,
        &[before, after.clone()],
    );
    let full = texture();
    render(&mut WgpuRenderer::new().unwrap(), &full, &[after]);

    assert!(
        redrawn < u64::from(W * H) / 2,
        "only the changed region is redrawn, got {redrawn} of {} pixels",
        W * H
    );
    let (partial, full) = (pixels(&partial), pixels(&full));
    for (index, (a, b)) in partial.iter().zip(&full).enumerate() {
        assert!(
            (i32::from(*a) - i32::from(*b)).abs() <= 1,
            "pixel byte {index} differs: region redraw {a}, full frame {b}"
        );
    }
}

fn grouped(clip_width: f32, opacity: f32) -> CommandScene {
    let mut b = SceneBuilder::new();
    b.draw_rect(
        Rect::new(Point::new(0.0, 0.0), W as f32, H as f32),
        style(20, 20, 30),
    );
    b.with_clip(Rect::new(Point::new(8.0, 8.0), clip_width, 32.0), |b| {
        b.draw_rect(
            Rect::new(Point::new(8.0, 8.0), 48.0, 32.0),
            style(200, 200, 40),
        );
    });
    b.with_layer(opacity, |b| {
        b.draw_rect(
            Rect::new(Point::new(8.0, 48.0), 32.0, 32.0),
            style(40, 160, 220),
        );
    });
    b.finish()
}

#[test]
#[ignore = "needs a wgpu adapter; run with --ignored on GPU hardware (see README verification)"]
fn a_changed_clip_or_group_opacity_repaints_the_content_it_covers() {
    if sigil_kittest::gpu::shared_device()
        .expect("sigil-kittest")
        .is_none()
    {
        return;
    }
    let before = grouped(48.0, 1.0);
    let after = grouped(20.0, 0.4);

    let partial = texture();
    let redrawn = render(
        &mut WgpuRenderer::new().unwrap(),
        &partial,
        &[before, after.clone()],
    );
    let full = texture();
    render(&mut WgpuRenderer::new().unwrap(), &full, &[after]);

    assert!(
        redrawn < u64::from(W * H) / 2,
        "only the clip and the group are redrawn, got {redrawn} of {} pixels",
        W * H
    );
    let (partial, full) = (pixels(&partial), pixels(&full));
    for (index, (a, b)) in partial.iter().zip(&full).enumerate() {
        assert!(
            (i32::from(*a) - i32::from(*b)).abs() <= 1,
            "pixel byte {index} differs: region redraw {a}, full frame {b}"
        );
    }
}

fn list(highlight: Option<usize>) -> CommandScene {
    let mut b = SceneBuilder::new();
    b.draw_rect(
        Rect::new(Point::new(0.0, 0.0), W as f32, H as f32),
        style(20, 20, 30),
    );
    for row in 0..6_u8 {
        let y = f32::from(row) * 12.0;
        if highlight == Some(usize::from(row)) {
            b.draw_rect(
                Rect::new(Point::new(0.0, y), W as f32, 12.0),
                style(60, 60, 90),
            );
        }
        b.draw_rect(
            Rect::new(Point::new(4.0, y + 3.0), 40.0, 6.0),
            style(180, 180, 180),
        );
    }
    b.with_clip(Rect::new(Point::new(0.0, 80.0), W as f32, 48.0), |b| {
        b.draw_rect(
            Rect::new(Point::new(8.0, 88.0), 64.0, 24.0),
            style(40, 160, 220),
        );
    });
    b.finish()
}

#[test]
#[ignore = "needs a wgpu adapter; run with --ignored on GPU hardware (see README verification)"]
fn an_inserted_highlight_redraws_only_its_row() {
    if sigil_kittest::gpu::shared_device()
        .expect("sigil-kittest")
        .is_none()
    {
        return;
    }
    let before = list(None);
    let after = list(Some(2));

    let partial = texture();
    let redrawn = render(
        &mut WgpuRenderer::new().unwrap(),
        &partial,
        &[before, after.clone()],
    );
    let full = texture();
    render(&mut WgpuRenderer::new().unwrap(), &full, &[after]);

    assert!(
        redrawn < u64::from(W * H) / 8,
        "only the highlighted row is redrawn, got {redrawn} of {} pixels",
        W * H
    );
    let (partial, full) = (pixels(&partial), pixels(&full));
    for (index, (a, b)) in partial.iter().zip(&full).enumerate() {
        assert!(
            (i32::from(*a) - i32::from(*b)).abs() <= 1,
            "pixel byte {index} differs: region redraw {a}, full frame {b}"
        );
    }
}
