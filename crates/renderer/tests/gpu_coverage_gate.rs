//! Fail loudly when the GPU-backed tests did not actually execute.
//!
//! 34 call sites across 20 renderer test binaries follow this shape:
//!
//! ```ignore
//! let Some((device, queue)) = try_request_device("…") else {
//!     println!("no adapter — skipping …");
//!     return;
//! };
//! ```
//!
//! libtest has no runtime "skipped" state, so every one of those reports as
//! PASSED. On a machine with no adapter the suite therefore goes green with
//! 34 tests certifying nothing — the pixel checks, the atlas growth
//! crossings, the layer-composite contracts, all of it.
//!
//! Rather than rewrite 20 binaries (each with its own duplicated
//! `try_request_device`), this converts that silent condition into one loud
//! failure: if no adapter is present, the run cannot pass unless the
//! operator says so explicitly.
//!
//! Default is STRICT, deliberately. Sigil verifies on owned native builders
//! with real GPUs (see docs/plans/engineering-rules.md, local only), so the common
//! case must not be the unchecked one. A genuinely GPU-less environment
//! opts out with `SIGIL_ALLOW_NO_GPU=1`, which is a visible, greppable
//! statement that this run's GPU coverage is void.

#![allow(missing_docs)]
#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::print_stdout)]

use pollster::FutureExt;

/// Number of `no adapter — skipping` sites at the time of writing. Purely
/// informational in the failure message; it is not asserted, so it does not
/// become a maintenance burden as tests come and go.
const APPROX_GPU_SKIP_SITES: u32 = 34;

fn adapter_available() -> bool {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            compatible_surface: None,
            force_fallback_adapter: false,
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
