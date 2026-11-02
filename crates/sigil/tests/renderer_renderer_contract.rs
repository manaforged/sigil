#![cfg(feature = "kittest")]
#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]

use std::time::{Duration, Instant};

use sigil::hal::Renderer;
use sigil::renderer::WgpuRenderer;

const SLUG_SHADER: &str = include_str!("../src/renderer/slug/shader.wgsl");

#[test]
fn renderer_error_is_send_sync_static() {
    fn assert_send_sync_static<T: Send + Sync + 'static>() {}
    assert_send_sync_static::<sigil::Error>();
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
fn renderer_new_does_not_block_on_gpu() {
    let start = Instant::now();
    drop(WgpuRenderer::new());
    let elapsed = start.elapsed();

    assert!(
        elapsed < Duration::from_secs(1),
        "WgpuRenderer::new() should not block on adapter/device selection \
         (expected < 1 s, got {elapsed:?}). If this test regresses, adapter \
         or device work has leaked back out of create_surface."
    );
}
