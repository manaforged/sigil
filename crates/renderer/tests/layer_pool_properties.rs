//! Property-style coverage for Phase R11 `LayerPool` key-cache behavior.

#![allow(missing_docs)]
#![allow(clippy::unwrap_used, clippy::panic)]
#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
#![allow(unused_crate_dependencies)]

use ahash as _;
use bytemuck as _;
use criterion as _;
use half as _;
use image as _;
use proptest as _;
use raw_window_handle as _;
use sigil_hal as _;
use sigil_icons as _;
use sigil_kernel as _;
use sigil_layout as _;
use sigil_shell as _;
use sigil_style as _;
use sigil_text as _;
use skrifa as _;
use smallvec as _;
use tracing as _;
#[cfg(windows)]
use windows_core as _;

use pollster::FutureExt as _;
use sigil_renderer::LayerPool;
use zeno as _;

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

#[test]
fn prop_acquire_returns_rounded_dims() {
    let Some(device) = try_request_device() else {
        eprintln!("no adapter - skipping layer_pool_properties::prop_acquire_returns_rounded_dims");
        return;
    };
    let mut pool = LayerPool::new();

    for (w, h) in [(1, 1), (15, 17), (127, 129), (2047, 1025)] {
        let handle = pool.acquire(&device, w, h, FORMAT).expect("layer handle");
        assert_eq!(handle.width, round_up(w, 16));
        assert_eq!(handle.height, round_up(h, 16));
        assert_eq!(handle.format, FORMAT);
    }
}

#[test]
fn prop_cache_hit_on_matching_dims() {
    let Some(device) = try_request_device() else {
        eprintln!("no adapter - skipping layer_pool_properties::prop_cache_hit_on_matching_dims");
        return;
    };
    let mut pool = LayerPool::new();

    let first = pool.acquire(&device, 31, 47, FORMAT).expect("first");
    assert_eq!(pool.len(), 1);
    let second = pool.acquire(&device, 32, 48, FORMAT).expect("second");
    assert_eq!(pool.len(), 1, "rounded matching dimensions reuse one key");
    assert_eq!((first.width, first.height), (second.width, second.height));
}

#[test]
fn prop_distinct_dims_distinct_entries() {
    let Some(device) = try_request_device() else {
        eprintln!(
            "no adapter - skipping layer_pool_properties::prop_distinct_dims_distinct_entries"
        );
        return;
    };
    let mut pool = LayerPool::new();

    let a = pool.acquire(&device, 16, 16, FORMAT).expect("a");
    let b = pool.acquire(&device, 33, 16, FORMAT).expect("b");
    assert_ne!((a.width, a.height), (b.width, b.height));
    assert_eq!(pool.len(), 2);
}

#[test]
fn prop_lru_evicts_oldest_at_capacity() {
    let Some(device) = try_request_device() else {
        eprintln!(
            "no adapter - skipping layer_pool_properties::prop_lru_evicts_oldest_at_capacity"
        );
        return;
    };
    let mut pool = LayerPool::new();

    for width in [16, 32, 48, 64] {
        let _ = pool
            .acquire(&device, width, 16, FORMAT)
            .expect("warm entry");
        pool.next_frame();
    }
    assert_eq!(pool.len(), LayerPool::MAX_ENTRIES);

    let _ = pool
        .acquire(&device, 80, 16, FORMAT)
        .expect("evicting entry");
    assert_eq!(pool.len(), LayerPool::MAX_ENTRIES);

    let _ = pool
        .acquire(&device, 16, 16, FORMAT)
        .expect("reinsert first key");
    assert_eq!(
        pool.len(),
        LayerPool::MAX_ENTRIES,
        "reinsert after eviction keeps cache at capacity"
    );
}

#[test]
fn prop_format_keyed() {
    let Some(device) = try_request_device() else {
        eprintln!("no adapter - skipping layer_pool_properties::prop_format_keyed");
        return;
    };
    let mut pool = LayerPool::new();

    let srgb = pool.acquire(&device, 120, 80, FORMAT).expect("srgb");
    let linear = pool
        .acquire(&device, 120, 80, wgpu::TextureFormat::Rgba8Unorm)
        .expect("linear");
    assert_eq!(pool.len(), 2);
    assert_eq!(srgb.format, FORMAT);
    assert_eq!(linear.format, wgpu::TextureFormat::Rgba8Unorm);
    assert_eq!((srgb.width, srgb.height), (linear.width, linear.height));
}

#[test]
fn prop_clear_drops_all_entries() {
    let Some(device) = try_request_device() else {
        eprintln!("no adapter - skipping layer_pool_properties::prop_clear_drops_all_entries");
        return;
    };
    let mut pool = LayerPool::new();

    for width in [16, 32, 48, 64] {
        let _ = pool.acquire(&device, width, 32, FORMAT).expect("entry");
    }
    assert_eq!(pool.len(), LayerPool::MAX_ENTRIES);
    pool.clear();
    assert_eq!(pool.len(), 0);
    assert!(pool.is_empty());

    let handle = pool
        .acquire(&device, 16, 32, FORMAT)
        .expect("fresh after clear");
    assert_eq!((handle.width, handle.height), (16, 32));
    assert_eq!(pool.len(), 1);
}

#[test]
fn prop_oversized_returns_none() {
    let Some(device) = try_request_device() else {
        eprintln!("no adapter - skipping layer_pool_properties::prop_oversized_returns_none");
        return;
    };
    let mut pool = LayerPool::new();
    let max = device.limits().max_texture_dimension_2d;

    assert!(
        pool.acquire(&device, max.saturating_add(1), 16, FORMAT)
            .is_none()
    );
    assert!(
        pool.acquire(&device, 16, max.saturating_add(1), FORMAT)
            .is_none()
    );
    assert_eq!(pool.len(), 0);
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
