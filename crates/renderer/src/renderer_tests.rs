use super::{PhysicalSize, select_alpha_mode, surface_extent};
use wgpu::CompositeAlphaMode as Mode;

#[test]
fn surface_extent_clamps_to_adapter_limit() {
    assert_eq!(
        surface_extent(PhysicalSize::new(2864, 1524), 2048),
        (2048, 1524),
    );
}

#[test]
fn surface_extent_never_returns_zero() {
    assert_eq!(surface_extent(PhysicalSize::new(0, 0), 2048), (1, 1));
}

#[test]
fn transparent_prefers_post_multiplied() {
    // Typical Metal offering.
    let metal_modes = [Mode::Opaque, Mode::PostMultiplied];
    assert_eq!(select_alpha_mode(&metal_modes, true), Mode::PostMultiplied);
    // Vulkan-style premultiplied-only offering.
    let vulkan_modes = [Mode::Opaque, Mode::PreMultiplied];
    assert_eq!(select_alpha_mode(&vulkan_modes, true), Mode::PreMultiplied);
}

#[test]
fn transparent_falls_back_to_opaque_when_unsupported() {
    assert_eq!(select_alpha_mode(&[Mode::Opaque], true), Mode::Opaque);
    assert_eq!(select_alpha_mode(&[], true), Mode::Auto);
}

#[test]
fn opaque_path_keeps_first_offering() {
    let modes = [Mode::Opaque, Mode::PostMultiplied];
    assert_eq!(select_alpha_mode(&modes, false), Mode::Opaque);
    assert_eq!(select_alpha_mode(&[], false), Mode::Auto);
}
