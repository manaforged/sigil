//! Contract tests for [`renderer::WgpuRenderer`].
//!
//! A real-GPU test suite is not practical in CI — we don't know what
//! hardware runners have. We split the tests into two tiers:
//!
//! 1. **Unconditional** tests that exercise pure types (error formatting,
//!    trait shape). These always run.
//! 2. **GPU-gated** tests that construct a `WgpuRenderer`. These are
//!    marked `#[ignore]` by default so `cargo test` is green on
//!    runners without a GPU; run locally with
//!    `cargo test -p sigil-renderer -- --ignored` on a machine with a
//!    working adapter.
//!
//! We deliberately do NOT add a cargo feature gate: the `#[ignore]`
//! attribute gives the exact opt-in semantics we want without pulling
//! a feature flag through every workspace build.

// Integration tests get linked as a standalone crate that inherits the
// library's direct deps but doesn't name most of them. Silence the
// workspace `unused_crate_dependencies` lint for the ones only the
// library's production code uses.
use ahash as _;
use bytemuck as _;
use criterion as _;
use half as _;
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
use wgpu as _;
#[cfg(windows)]
use windows_core as _;

use icons as _;

use core::error::Error;
use std::time::{Duration, Instant};

use hal::{Color, PhysicalSize, Renderer};
use renderer::{RendererError, WgpuRenderer};

const SLUG_SHADER: &str = include_str!("../src/slug/shader.wgsl");

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

/// Every `RendererError` variant must produce non-empty `Display`
/// output and report an `Error::source` that matches the variant
/// shape. This is the floor for log-debuggability; regressions here
/// mean operators see empty strings when the renderer fails.
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

    // `NoSupportedFormat` and `NotInitialized` were added when the
    // renderer moved to lazy adapter/device selection. Both must
    // produce a non-empty Display and carry no inner cause.
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

/// `RendererError` must satisfy the HAL's `Send + Sync + 'static`
/// contract so it can be returned from trait methods across threads.
/// This is a compile-time assertion dressed up as a test.
#[test]
fn renderer_error_is_send_sync_static() {
    fn assert_send_sync_static<T: Send + Sync + 'static>() {}
    assert_send_sync_static::<RendererError>();
}

/// `WgpuRenderer` must implement `hal::Renderer` with the expected
/// associated types. Pure-types check — no GPU needed.
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

/// The Slug shader receives UVs at fragment centers, so the analytic
/// edge ramp must include the half-pixel bias from the center to the
/// pixel edge. This source-level guard keeps the WGSL helper aligned
/// with the math regression below.
#[test]
fn slug_shader_edge_ramp_accounts_for_fragment_centers() {
    let expected = "return clamp((edge_x - sample_x) * width + 0.5, 0.0, 1.0);";
    assert!(SLUG_SHADER.contains("fn edge_coverage("));
    assert!(
        SLUG_SHADER.contains(expected),
        "Slug shader edge ramp must keep the +0.5 center-sample bias"
    );
}

/// A one-pixel stem whose edge falls on pixel boundaries should render
/// its interior center samples at full coverage. Without the +0.5 edge
/// bias, the pixel just inside the right edge evaluates to 0.5 alpha.
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

/// A freshly-constructed renderer on a GPU-capable host must succeed.
/// Gated because CI runners often have no adapter.
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

/// `WgpuRenderer::new` must not touch an adapter or device — that work
/// is deferred to the first `create_surface` call so we pick a GPU
/// that is known-compatible with the user's window.
///
/// We express "does not block on the GPU" as a wall-clock budget that
/// is loose enough to absorb first-call backend-DLL loading on Windows
/// debug builds (observed ~170 ms) but tight enough to fail loudly if
/// adapter enumeration or device creation leaks back out of
/// `create_surface`. On a dGPU laptop, `request_adapter` alone can
/// take 500 ms to 2 s while the discrete GPU spins up, so a 1 s
/// budget still catches regressions without flaking on cold Windows
/// machines.
///
/// This test runs unconditionally: it should pass on CI runners with
/// no GPU because `new` never asks for one.
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

/// `resize` on a zero-sized dimension is a documented no-op. Gated
/// because we need a real surface to exercise the path, which needs a
/// real window handle and therefore a real GPU pipeline end-to-end.
///
/// This test is a placeholder for the real end-to-end exercise the
/// `hello-rect` rewrite will provide. Leaving it here as a reminder
/// that the zero-size invariant is part of the trait contract, not
/// just `WgpuRenderer`'s convenience.
#[test]
#[ignore = "needs a real windowed surface; exercised by the hello-rect rewrite"]
fn zero_sized_resize_is_noop() {
    // Constructing a surface requires a `WindowHandle`, which in turn
    // requires an OS window. Event loops can't spin inside a cargo
    // test reliably across platforms. The contract is exercised by
    // `examples/hello-rect` once it is ported onto `sigil-renderer`.
    let _ = PhysicalSize::new(0, 0);
    let _ = Color::BLACK;
}
