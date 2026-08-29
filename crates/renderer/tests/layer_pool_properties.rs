//! Coverage for [`LayerPool`] reuse and fail-closed acquire.

#![allow(missing_docs)]
#![allow(clippy::unwrap_used, clippy::panic)]
#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]

use pollster::FutureExt;
use sigil_renderer::{LayerError, LayerPool};

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

#[test]
#[ignore = "needs a wgpu adapter; run with --ignored on GPU hardware (see README verification)"]
fn acquire_returns_rounded_dims() {
    let Some(device) = try_request_device() else {
        eprintln!("no adapter - skipping acquire_returns_rounded_dims");
        return;
    };
    let mut pool = LayerPool::new();

    for (w, h) in [(1, 1), (15, 17), (127, 129), (2047, 1025)] {
        let handle = pool.acquire(&device, w, h, FORMAT).expect("layer handle");
        assert_eq!(handle.width, round_up(w, 16));
        assert_eq!(handle.height, round_up(h, 16));
        assert_eq!(handle.format, FORMAT);
        pool.release(handle);
    }
}

#[test]
#[ignore = "needs a wgpu adapter; run with --ignored on GPU hardware (see README verification)"]
fn release_then_acquire_reuses_same_size() {
    let Some(device) = try_request_device() else {
        eprintln!("no adapter - skipping release_then_acquire_reuses_same_size");
        return;
    };
    let mut pool = LayerPool::new();

    let first = pool.acquire(&device, 31, 47, FORMAT).expect("first");
    assert!(pool.is_empty());
    pool.release(first);
    assert_eq!(pool.len(), 1);
    let second = pool.acquire(&device, 32, 48, FORMAT).expect("reuse");
    assert!(
        pool.is_empty(),
        "matching rounded size leaves the free list"
    );
    assert_eq!((second.width, second.height), (32, 48));
    pool.release(second);
}

#[test]
#[ignore = "needs a wgpu adapter; run with --ignored on GPU hardware (see README verification)"]
fn free_list_caps_at_max_entries() {
    let Some(device) = try_request_device() else {
        eprintln!("no adapter - skipping free_list_caps_at_max_entries");
        return;
    };
    let mut pool = LayerPool::new();
    for width in [16, 32, 48, 64, 80] {
        let handle = pool.acquire(&device, width, 16, FORMAT).expect("warm");
        pool.release(handle);
    }
    assert_eq!(pool.len(), LayerPool::MAX_ENTRIES);
}

#[test]
#[ignore = "needs a wgpu adapter; run with --ignored on GPU hardware (see README verification)"]
fn oversized_returns_err() {
    let Some(device) = try_request_device() else {
        eprintln!("no adapter - skipping oversized_returns_err");
        return;
    };
    let mut pool = LayerPool::new();
    let max = device.limits().max_texture_dimension_2d;

    assert!(matches!(
        pool.acquire(&device, max.saturating_add(1), 16, FORMAT),
        Err(LayerError::TooLarge { .. })
    ));
    assert!(matches!(
        pool.acquire(&device, 16, max.saturating_add(1), FORMAT),
        Err(LayerError::TooLarge { .. })
    ));
    assert_eq!(pool.len(), 0);
}

#[test]
#[ignore = "needs a wgpu adapter; run with --ignored on GPU hardware (see README verification)"]
fn zero_extent_returns_empty() {
    let mut pool = LayerPool::new();
    // Device is unused on the empty path.
    let Some(device) = try_request_device() else {
        eprintln!("no adapter - skipping zero_extent_returns_empty");
        return;
    };
    assert!(matches!(
        pool.acquire(&device, 0, 16, FORMAT),
        Err(LayerError::Empty)
    ));
}

fn try_request_device() -> Option<wgpu::Device> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            compatible_surface: None,
            force_fallback_adapter: false,
        })
        .block_on()
        .ok()?;
    adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: Some("layer-pool-properties test device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::downlevel_defaults(),
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
            memory_hints: wgpu::MemoryHints::Performance,
            trace: wgpu::Trace::Off,
        })
        .block_on()
        .ok()
        .map(|(device, _queue)| device)
}

#[allow(clippy::manual_div_ceil)]
const fn round_up(n: u32, multiple: u32) -> u32 {
    ((n + multiple - 1) / multiple) * multiple
}
