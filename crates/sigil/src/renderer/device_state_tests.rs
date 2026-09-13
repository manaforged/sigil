use super::renderer_limits;

#[test]
fn renderer_uses_adapter_resolution_for_the_swapchain() {
    let mut adapter = wgpu::Limits::downlevel_defaults();
    adapter.max_texture_dimension_1d = 8_192;
    adapter.max_texture_dimension_2d = 16_384;
    adapter.max_texture_dimension_3d = 2_048;

    let actual = renderer_limits(adapter);

    assert_eq!(actual.max_texture_dimension_1d, 8_192);
    assert_eq!(actual.max_texture_dimension_2d, 16_384);
    assert_eq!(actual.max_texture_dimension_3d, 2_048);
}
