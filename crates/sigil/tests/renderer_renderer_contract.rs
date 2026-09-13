#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]

use core::error::Error;
use std::time::{Duration, Instant};

use sigil::hal::{Color, PhysicalSize, Renderer};
use sigil::renderer::{RendererError, WgpuRenderer};

const SLUG_SHADER: &str = include_str!("../src/renderer/slug/shader.wgsl");

#[derive(Copy, Clone)]
struct StemRect {
    left: f32,
    right: f32,
    top: f32,
    bottom: f32,
}

fn centered_edge_coverage(edge_px: f32, sample_center_px: f32) -> f32 {
    (edge_px - sample_center_px + 0.5).clamp(0.0, 1.0)
}

fn stem_coverage(stem: StemRect, sample_x: f32, sample_y: f32) -> f32 {
    if sample_y < stem.top || sample_y >= stem.bottom {
        return 0.0;
    }
    let left = centered_edge_coverage(stem.left, sample_x);
    let right = centered_edge_coverage(stem.right, sample_x);
    (right - left).abs()
}

fn assert_near(actual: f32, expected: f32) {
    let delta = (actual - expected).abs();
    assert!(
        delta < 1e-6,
        "expected coverage {expected}, got {actual} (delta {delta})"
    );
}

#[test]
fn renderer_error_display_format() {
    let no_adapter = RendererError::NoAdapter;
    let rendered = format!("{no_adapter}");
    assert!(
        rendered.contains("no compatible GPU adapter"),
        "NoAdapter Display should mention the missing adapter; got: {rendered:?}"
    );
    assert!(
        no_adapter.source().is_none(),
        "NoAdapter has no inner cause"
    );

    let validation = RendererError::SurfaceValidation;
    assert!(!format!("{validation}").is_empty());
    assert!(validation.source().is_none());

    let no_format = RendererError::NoSupportedFormat;
    let rendered_no_format = format!("{no_format}");
    assert!(
        rendered_no_format.contains("no usable swapchain formats"),
        "NoSupportedFormat Display should describe the missing format list; got: {rendered_no_format:?}"
    );
    assert!(no_format.source().is_none());

    let not_init = RendererError::NotInitialized;
    let rendered_not_init = format!("{not_init}");
    assert!(
        rendered_not_init.contains("create_surface"),
        "NotInitialized Display should point callers at create_surface; got: {rendered_not_init:?}"
    );
    assert!(not_init.source().is_none());
}

#[test]
fn renderer_error_is_send_sync_static() {
    fn assert_send_sync_static<T: Send + Sync + 'static>() {}
    assert_send_sync_static::<RendererError>();
}

#[test]
fn renderer_associated_types_match_hal_contract() {
    fn assert_impls<R>()
    where
        R: Renderer,
        R::Error: core::error::Error + Send + Sync + 'static,
    {
    }
    assert_impls::<WgpuRenderer>();
}

#[test]
fn slug_shader_edge_ramp_accounts_for_fragment_centers() {
    let expected = "return clamp((edge_x - sample_x) * width + 0.5, 0.0, 1.0);";
    assert!(SLUG_SHADER.contains("fn edge_coverage("));
    assert!(
        SLUG_SHADER.contains(expected),
        "Slug shader edge ramp must keep the +0.5 center-sample bias"
    );
}

#[test]
fn slug_aa_keeps_one_pixel_stem_centers_opaque() {
    let vertical = StemRect {
        left: 1.0,
        right: 2.0,
        top: 0.0,
        bottom: 3.0,
    };
    assert_near(stem_coverage(vertical, 1.5, 0.5), 1.0);
    assert_near(stem_coverage(vertical, 0.5, 0.5), 0.0);
    assert_near(stem_coverage(vertical, 2.5, 0.5), 0.0);

    let horizontal = StemRect {
        left: 0.0,
        right: 3.0,
        top: 1.0,
        bottom: 2.0,
    };
    assert_near(stem_coverage(horizontal, 2.5, 1.5), 1.0);
}

#[test]
#[ignore = "requires a working GPU adapter; run locally with --ignored"]
fn wgpu_renderer_constructs_without_panic() {
    let renderer = WgpuRenderer::new();
    assert!(
        renderer.is_ok(),
        "WgpuRenderer::new() should succeed on a GPU-capable host, got {:?}",
        renderer.err()
    );
}

#[test]
fn renderer_new_does_not_block_on_gpu() {
    let start = Instant::now();
    let result = WgpuRenderer::new();
    let elapsed = start.elapsed();

    assert!(
        result.is_ok(),
        "WgpuRenderer::new() should succeed without a GPU (only creates the wgpu instance); \
         got {:?}",
        result.err()
    );
    assert!(
        elapsed < Duration::from_secs(1),
        "WgpuRenderer::new() should not block on adapter/device selection \
         (expected < 1 s, got {elapsed:?}). If this test regresses, adapter \
         or device work has leaked back out of create_surface."
    );
}

#[test]
#[ignore = "needs a real windowed surface; exercised by the hello-rect rewrite"]
fn zero_sized_resize_is_noop() {
    let _ = PhysicalSize::new(0, 0);
    let _ = Color::BLACK;
}
