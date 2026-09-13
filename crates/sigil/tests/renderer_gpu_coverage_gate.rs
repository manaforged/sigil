#![allow(missing_docs)]
#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::print_stdout)]

use pollster::FutureExt;

const APPROX_GPU_SKIP_SITES: u32 = 34;

fn adapter_available() -> bool {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            compatible_surface: None,
            force_fallback_adapter: false,
            apply_limit_buckets: false,
        })
        .block_on()
        .is_ok()
}

#[test]
#[ignore = "needs a wgpu adapter; run with --ignored on GPU hardware (see README verification)"]
fn gpu_backed_tests_actually_ran() {
    if adapter_available() {
        return;
    }
    assert!(
        std::env::var("SIGIL_ALLOW_NO_GPU").as_deref() == Ok("1"),
        "no wgpu adapter is available, so roughly {APPROX_GPU_SKIP_SITES} GPU-backed \
         tests returned early and reported PASSED without executing — every pixel \
         check, atlas-growth crossing and layer-composite contract among them.\n\
         This run's GPU coverage is void. Verify on a builder with a real adapter, \
         or set SIGIL_ALLOW_NO_GPU=1 to acknowledge running without it.",
    );
    println!("SIGIL_ALLOW_NO_GPU=1: ~{APPROX_GPU_SKIP_SITES} GPU-backed tests did not execute");
}
