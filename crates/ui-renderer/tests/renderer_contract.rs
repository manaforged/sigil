//! Contract tests for [`ui_renderer::WgpuRenderer`].
//!
//! A real-GPU test suite is not practical in CI — we don't know what
//! hardware runners have. We split the tests into two tiers:
//!
//! 1. **Unconditional** tests that exercise pure types (error formatting,
//!    trait shape). These always run.
//! 2. **GPU-gated** tests that construct a `WgpuRenderer`. These are
//!    marked `#[ignore]` by default so `cargo test` is green on
//!    runners without a GPU; run locally with
//!    `cargo test -p ui-renderer -- --ignored` on a machine with a
//!    working adapter.
//!
//! We deliberately do NOT add a cargo feature gate: the `#[ignore]`
//! attribute gives the exact opt-in semantics we want without pulling
//! a feature flag through every workspace build.

// Integration tests get linked as a standalone crate that inherits the
// library's direct deps but doesn't name most of them. Silence the
// workspace `unused_crate_dependencies` lint for the ones only the
// library's production code uses.
use pollster as _;
use raw_window_handle as _;
use wgpu as _;
#[cfg(windows)]
use windows_core as _;

use core::error::Error;
use std::time::{Duration, Instant};

use ui_hal::{Color, PhysicalSize, Renderer};
use ui_renderer::{RendererError, WgpuRenderer};

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
    let rendered = format!("{no_format}");
    assert!(
        rendered.contains("no usable swapchain formats"),
        "NoSupportedFormat Display should describe the missing format list; got: {rendered:?}"
    );
    assert!(no_format.source().is_none());

    let not_init = RendererError::NotInitialized;
    let rendered = format!("{not_init}");
    assert!(
        rendered.contains("create_surface"),
        "NotInitialized Display should point callers at create_surface; got: {rendered:?}"
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

/// `WgpuRenderer` must implement `ui_hal::Renderer` with the expected
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
    // `examples/hello-rect` once it is ported onto `ui-renderer`.
    let _ = PhysicalSize::new(0, 0);
    let _ = Color::BLACK;
}
