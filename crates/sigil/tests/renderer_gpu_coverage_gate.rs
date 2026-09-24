#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::print_stdout)]

const APPROX_GPU_SKIP_SITES: u32 = 34;

#[test]
#[ignore = "needs a wgpu adapter; run with --ignored on GPU hardware (see README verification)"]
fn gpu_backed_tests_actually_ran() {
    let gpu = sigil_kittest::gpu::shared_device();
    if matches!(gpu, Ok(Some(_))) {
        return;
    }
    assert!(
        gpu.is_ok(),
        "no wgpu adapter is available, so roughly {APPROX_GPU_SKIP_SITES} GPU-backed \
         tests returned early and reported PASSED without executing — every pixel \
         check, atlas-growth crossing and layer-composite contract among them.\n\
         This run's GPU coverage is void. Verify on a builder with a real adapter, \
         or set SIGIL_ALLOW_NO_GPU=1 to acknowledge running without it.",
    );
    println!("SIGIL_ALLOW_NO_GPU=1: ~{APPROX_GPU_SKIP_SITES} GPU-backed tests did not execute");
}
