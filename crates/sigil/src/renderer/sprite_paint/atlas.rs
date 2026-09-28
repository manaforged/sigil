use wgpu::util::DeviceExt as _;

use super::builder::atlas_bind_group;

pub(super) const MAX_ATLASES: usize = 16;

pub(crate) struct AtlasDesc<'a> {
    pub(crate) bytes: &'a [u8],
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) format: wgpu::TextureFormat,
}

pub(crate) struct AtlasEntry {
    pub(super) texture: wgpu::Texture,
    pub(super) width: u32,
    pub(super) height: u32,
}

#[allow(
    clippy::too_many_arguments,
    reason = "the argument list mirrors the upstream API contract"
)]
pub(super) fn push_atlas(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
    sampler: &wgpu::Sampler,
    atlases: &mut Vec<AtlasEntry>,
    atlas_bind_groups: &mut Vec<wgpu::BindGroup>,
    bytes: &[u8],
    width: u32,
    height: u32,
    format: wgpu::TextureFormat,
) {
    let desc = AtlasDesc {
        bytes,
        width,
        height,
        format,
    };
    let (entry, bind_group) = build_atlas(device, queue, layout, sampler, &desc);
    atlases.push(entry);
    atlas_bind_groups.push(bind_group);
}

pub(super) fn build_atlas(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
    sampler: &wgpu::Sampler,
    desc: &AtlasDesc<'_>,
) -> (AtlasEntry, wgpu::BindGroup) {
    let texture = device.create_texture_with_data(
        queue,
        &wgpu::TextureDescriptor {
            label: Some("sigil-renderer sprite atlas"),
            size: wgpu::Extent3d {
                width: desc.width,
                height: desc.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: desc.format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        },
        wgpu::util::TextureDataOrder::LayerMajor,
        desc.bytes,
    );
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let bind_group = atlas_bind_group(device, layout, &view, sampler);
    let entry = AtlasEntry {
        texture,
        width: desc.width,
        height: desc.height,
    };
    (entry, bind_group)
}

pub(super) fn valid_atlas_desc(
    bytes: &[u8],
    width: u32,
    height: u32,
    format: wgpu::TextureFormat,
) -> bool {
    if bytes.is_empty()
        || width == 0
        || height == 0
        || width > MAX_ATLAS_AXIS
        || height > MAX_ATLAS_AXIS
    {
        return false;
    }
    let bytes_per_pixel = bytes_per_pixel(format);
    if bytes_per_pixel == 0 {
        return false;
    }
    let Some(expected_len) = width
        .checked_mul(height)
        .and_then(|texels| texels.checked_mul(bytes_per_pixel))
        .and_then(|len| usize::try_from(len).ok())
    else {
        return false;
    };
    bytes.len() == expected_len
}

const MAX_ATLAS_AXIS: u32 = 8192;

const fn bytes_per_pixel(format: wgpu::TextureFormat) -> u32 {
    match format {
        wgpu::TextureFormat::Rgba8UnormSrgb | wgpu::TextureFormat::Rgba8Unorm => 4,
        wgpu::TextureFormat::R8Unorm => 1,
        _ => 0,
    }
}
