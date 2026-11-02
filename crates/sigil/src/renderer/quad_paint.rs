#![cfg_attr(not(any(test, feature = "kittest")), allow(unreachable_pub))]

use bytemuck::{Pod, Zeroable};
use std::collections::HashMap;

mod atlas;
mod builder;
mod icon;
mod image_pack;
mod kind;
mod mask;
mod mask_atlas;
mod prepare;

use crate::geometry::Rect;
use crate::renderer::fill_paint::setup::{
    QuadDesc, QuadPipeline, begin_pass, linear_sampler, texture_layout,
};
use crate::renderer::scene_command::{DrawSprite, Transform};
use crate::renderer::scene_instance::BlendSpace;
use crate::renderer::scene_paint::PaintCtx;
use crate::renderer::slug::RunFrame;
use crate::renderer::split_encoder::SplitEncoder;
use crate::renderer::sprite_atlas::{
    BUILTIN_ATLAS_HEIGHT, BUILTIN_ATLAS_WIDTH, BUILTIN_ICON_ATLAS_COUNT, builtin_atlas_rgba8,
    builtin_icon_index,
};

use self::atlas::{AtlasDesc, AtlasEntry, MAX_ATLASES, valid_atlas_desc};
use self::builder::{INSTANCE_LAYOUT, uniform_bind_group, uniform_layout};
use self::image_pack::ImagePack;
pub(crate) use self::image_pack::PackedImage;
pub(crate) use self::kind::QuadBlend;
pub(crate) use self::kind::{QuadInstance, QuadKind};
pub(crate) use self::mask::MaskParams;
use self::mask_atlas::RasterAtlas;
#[cfg(any(test, feature = "kittest"))]
pub use self::mask_atlas::set_test_atlas_size_override;
#[cfg(any(test, feature = "kittest"))]
pub(crate) use self::mask_atlas::test_atlas_size_override;
pub(crate) use self::prepare::raster_scale;
use crate::renderer::raster::Hinters;

pub const BUILTIN_SPRITE_ATLAS_ID: u32 = u32::MAX - 1;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct QuadGroup {
    pub(crate) clip: Option<Rect>,
    pub(crate) range: core::ops::Range<u32>,
}

pub(crate) struct QuadBatch<'a> {
    pub(crate) atlas: &'a wgpu::BindGroup,
    pub(crate) atlas_size: (f32, f32),
    pub(crate) blend: QuadBlend,
    pub(crate) instances: &'a [QuadInstance],
    pub(crate) groups: &'a [QuadGroup],
}

#[derive(Clone, Copy)]
pub(crate) struct SpritePaint {
    pub(crate) opacity: f32,
    pub(crate) space: BlendSpace,
    pub(crate) transform: Transform,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct QuadUniforms {
    viewport: [f32; 2],
    atlas_size: [f32; 2],
}

pub(crate) struct AtlasPipeline {
    straight: QuadPipeline,
    premultiplied: QuadPipeline,
    uniform_layout: wgpu::BindGroupLayout,
    uniforms: Option<(wgpu::Buffer, wgpu::BindGroup)>,
    atlas_layout: wgpu::BindGroupLayout,
    pub(super) atlases: Vec<AtlasEntry>,
    atlas_bind_groups: Vec<wgpu::BindGroup>,
    sampler: wgpu::Sampler,
    mask_sampler: wgpu::Sampler,
    mask_atlas: RasterAtlas,
    mask_group: Option<(wgpu::TextureView, wgpu::BindGroup)>,
    hinters: Hinters,
    builtin_atlas_slot: Option<usize>,
    builtin_icon_slots: [Option<usize>; BUILTIN_ICON_ATLAS_COUNT],
    pub(super) image_slots: HashMap<u64, PackedImage>,
    pub(super) image_pack: ImagePack,
}

fn quad_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    groups: &[Option<&wgpu::BindGroupLayout>],
    blend: QuadBlend,
) -> QuadPipeline {
    QuadPipeline::new(
        device,
        format,
        QuadDesc {
            label: "sigil-renderer atlas quad",
            wgsl: QUAD_SHADER,
            groups,
            instance: INSTANCE_LAYOUT,
            fs_entry: "fs_main",
            blend: blend.state(),
        },
    )
}

impl AtlasPipeline {
    pub(crate) fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let uniform_layout = uniform_layout(device);
        let atlas_layout = texture_layout(device, "sigil-renderer atlas quad layout");
        let groups = [Some(&uniform_layout), Some(&atlas_layout)];
        Self {
            straight: quad_pipeline(device, format, &groups, QuadBlend::Straight),
            premultiplied: quad_pipeline(device, format, &groups, QuadBlend::Premultiplied),
            uniform_layout,
            uniforms: None,
            atlas_layout,
            atlases: Vec::new(),
            atlas_bind_groups: Vec::new(),
            sampler: linear_sampler(device, "sigil-renderer sprite sampler"),
            mask_sampler: mask::nearest_sampler(device),
            mask_atlas: RasterAtlas::new(device),
            mask_group: None,
            hinters: Hinters::default(),
            builtin_atlas_slot: None,
            builtin_icon_slots: [None; BUILTIN_ICON_ATLAS_COUNT],
            image_slots: HashMap::new(),
            image_pack: ImagePack::default(),
        }
    }

    pub(crate) const fn format(&self) -> wgpu::TextureFormat {
        self.premultiplied.format
    }

    pub(crate) fn register_atlas(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        desc: &AtlasDesc<'_>,
    ) -> u32 {
        if self.atlases.len() >= MAX_ATLASES || !valid_atlas_desc(desc) {
            return u32::MAX;
        }
        let id = u32::try_from(self.atlases.len()).unwrap_or(u32::MAX);
        self.push_atlas(device, queue, desc);
        id
    }

    pub(crate) fn paint_sprite(
        &mut self,
        ctx: &mut PaintCtx<'_>,
        draw: &DrawSprite,
        paint: SpritePaint,
    ) {
        let Some(atlas_id) = self.resolve_atlas(ctx.device, ctx.queue, draw.source.atlas) else {
            return;
        };
        let Some(instance) = self.build_instance(atlas_id, draw, paint) else {
            return;
        };
        let Some(atlas_size) = self.atlas_dims(atlas_id) else {
            return;
        };
        let Some(atlas) = self.atlas_bind_groups.get(atlas_id).cloned() else {
            return;
        };
        let frame = RunFrame {
            device: ctx.device,
            view: ctx.view,
            surface_width: ctx.viewport.0,
            surface_height: ctx.viewport.1,
            scale: ctx.scale,
        };
        self.draw_quads(
            ctx.encoder,
            &frame,
            &QuadBatch {
                atlas: &atlas,
                atlas_size,
                blend: QuadKind::from(draw.kind).blend(),
                instances: &[instance],
                groups: &[QuadGroup {
                    clip: ctx.clip,
                    range: 0..1,
                }],
            },
        );
    }

    fn resolve_atlas(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        raw_id: u32,
    ) -> Option<usize> {
        if builtin_icon_index(raw_id).is_some() {
            return self.resolve_builtin_icon_atlas(device, queue, raw_id);
        }
        if raw_id != BUILTIN_SPRITE_ATLAS_ID {
            return Some(raw_id as usize);
        }
        if let Some(slot) = self.builtin_atlas_slot {
            return Some(slot);
        }
        if self.atlases.len() >= MAX_ATLASES {
            return None;
        }
        let slot = self.atlases.len();
        self.push_atlas(
            device,
            queue,
            &AtlasDesc {
                bytes: &builtin_atlas_rgba8(),
                width: BUILTIN_ATLAS_WIDTH,
                height: BUILTIN_ATLAS_HEIGHT,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
            },
        );
        self.builtin_atlas_slot = Some(slot);
        Some(slot)
    }

    fn build_instance(
        &self,
        atlas_id: usize,
        draw: &DrawSprite,
        paint: SpritePaint,
    ) -> Option<QuadInstance> {
        let (aw, ah) = self.atlas_dims(atlas_id)?;
        let (r, dest) = draw.source.fitted((aw, ah), draw.destination)?;
        let uv_rect = [
            r.origin.x / aw,
            r.origin.y / ah,
            (r.origin.x + r.width) / aw,
            (r.origin.y + r.height) / ah,
        ];
        let opacity = (f32::from(draw.opacity) / 10_000.0 * paint.opacity).clamp(0.0, 1.0);
        Some(QuadInstance::new(
            (
                [dest.origin.x, dest.origin.y, dest.width, dest.height],
                uv_rect,
            ),
            paint.space.rgba(draw.color),
            (QuadKind::from(draw.kind), opacity, draw.source.phase),
            paint.transform,
        ))
    }

    fn atlas_dims(&self, atlas_id: usize) -> Option<(f32, f32)> {
        let entry = self.atlases.get(atlas_id)?;
        #[allow(
            clippy::cast_precision_loss,
            reason = "pixel and char-count magnitudes are bounded; sub-unit precision is irrelevant at render scale"
        )]
        Some((entry.width as f32, entry.height as f32))
    }

    fn bind_uniforms(&mut self, device: &wgpu::Device, uniform: wgpu::Buffer) {
        if self
            .uniforms
            .as_ref()
            .is_none_or(|(buffer, _)| *buffer != uniform)
        {
            let group = uniform_bind_group(device, &self.uniform_layout, &uniform);
            self.uniforms = Some((uniform, group));
        }
    }

    fn draw_quads(
        &mut self,
        encoder: &mut SplitEncoder,
        frame: &RunFrame<'_>,
        batch: &QuadBatch<'_>,
    ) {
        if batch.instances.is_empty() {
            return;
        }
        let uploaded = encoder.upload(batch.instances);
        let (uniform, offset, _) = encoder.uniform(&QuadUniforms {
            viewport: [frame.surface_width.max(1.0), frame.surface_height.max(1.0)],
            atlas_size: [batch.atlas_size.0, batch.atlas_size.1],
        });
        self.bind_uniforms(frame.device, uniform);
        let Some((_, uniform_group)) = self.uniforms.as_ref() else {
            return;
        };
        let quad = match batch.blend {
            QuadBlend::Straight => &self.straight,
            QuadBlend::Premultiplied => &self.premultiplied,
        };
        let size = frame.view.texture().size();
        let pass = begin_pass(
            encoder,
            batch.blend.pass_label(),
            frame.view,
            wgpu::LoadOp::Load,
        );
        let viewport = (frame.surface_width, frame.surface_height);
        let stride = size_of::<QuadInstance>() as u64;
        let start = uploaded.bytes.start;
        for group in batch.groups {
            if group.clip.is_none() {
                pass.set_scissor_rect(0, 0, size.width, size.height);
            }
            if crate::renderer::scissor::apply(pass, group.clip, frame.scale, viewport) {
                let bytes = start + u64::from(group.range.start) * stride
                    ..start + u64::from(group.range.end) * stride;
                quad.draw(
                    pass,
                    &[uniform_group, batch.atlas],
                    &[offset],
                    uploaded.buffer.slice(bytes),
                    group.range.end.saturating_sub(group.range.start),
                );
            }
        }
        pass.set_scissor_rect(0, 0, size.width, size.height);
    }
}

const _: () = assert!(core::mem::size_of::<QuadInstance>() == 96);
const _: () = assert!(core::mem::size_of::<QuadUniforms>() == 16);

const QUAD_SHADER: &str = include_str!("quad_paint.wgsl");

#[cfg(test)]
mod tests;
