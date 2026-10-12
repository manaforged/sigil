#![allow(
    clippy::redundant_pub_crate,
    dead_code,
    clippy::indexing_slicing,
    clippy::cast_precision_loss,
    clippy::imprecise_flops
)]
#![cfg_attr(not(any(test, feature = "kittest")), allow(unreachable_pub))]

use bytemuck::{Pod, Zeroable};
use std::slice::from_ref;

mod atlas;
mod builder;
mod icon;
mod image_pack;
mod prepare;

use crate::renderer::fill_paint::setup::{
    QuadDesc, QuadPipeline, clipped_pass, linear_sampler, texture_layout,
};
use crate::renderer::frame_upload::UploadedRange;
use crate::renderer::scene_command::{DrawSprite, SpriteKind, Transform};
use crate::renderer::scene_instance::{BlendSpace, affine_rows};
use crate::renderer::scene_paint::PaintCtx;
use crate::renderer::sprite_atlas::{
    BUILTIN_ATLAS_HEIGHT, BUILTIN_ATLAS_WIDTH, BUILTIN_ICON_ATLAS_COUNT, builtin_atlas_rgba8,
    builtin_icon_index,
};

use self::atlas::{AtlasDesc, AtlasEntry, MAX_ATLASES, valid_atlas_desc};
use self::builder::{INSTANCE_LAYOUT, uniform_bind_group, uniform_layout};
use self::image_pack::ImagePack;
pub(crate) use self::image_pack::PackedImage;
pub(crate) use self::prepare::raster_scale;

pub const BUILTIN_SPRITE_ATLAS_ID: u32 = u32::MAX - 1;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct SpriteInstance {
    dest: [f32; 4],
    uv_rect: [f32; 4],
    color: [f32; 4],
    params: [f32; 4],
    xform: [[f32; 4]; 2],
}

#[derive(Clone, Copy)]
pub(crate) struct SpritePaint {
    pub(crate) opacity: f32,
    pub(crate) space: BlendSpace,
    pub(crate) transform: Transform,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct SpriteUniforms {
    viewport: [f32; 2],
    atlas_size: [f32; 2],
}

pub(crate) struct SpritePipeline {
    quad: QuadPipeline,
    uniform_layout: wgpu::BindGroupLayout,
    uniforms: Option<(wgpu::Buffer, wgpu::BindGroup)>,
    atlas_layout: wgpu::BindGroupLayout,
    pub(super) atlases: Vec<AtlasEntry>,
    atlas_bind_groups: Vec<wgpu::BindGroup>,
    sampler: wgpu::Sampler,
    builtin_atlas_slot: Option<usize>,
    builtin_icon_slots: [Option<usize>; BUILTIN_ICON_ATLAS_COUNT],
    pub(super) image_slots: std::collections::HashMap<u64, PackedImage>,
    pub(super) image_pack: ImagePack,
}

impl SpritePipeline {
    pub(crate) fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let uniform_layout = uniform_layout(device);
        let atlas_layout = texture_layout(device, "sigil-renderer sprite atlas layout");
        let quad = QuadPipeline::new(
            device,
            format,
            QuadDesc {
                label: "sigil-renderer sprite",
                wgsl: SPRITE_SHADER,
                groups: &[Some(&uniform_layout), Some(&atlas_layout)],
                instance: INSTANCE_LAYOUT,
                fs_entry: "fs_main",
                blend: wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING,
            },
        );
        Self {
            quad,
            uniform_layout,
            uniforms: None,
            atlas_layout,
            atlases: Vec::new(),
            atlas_bind_groups: Vec::new(),
            sampler: linear_sampler(device, "sigil-renderer sprite sampler"),
            builtin_atlas_slot: None,
            builtin_icon_slots: [None; BUILTIN_ICON_ATLAS_COUNT],
            image_slots: std::collections::HashMap::new(),
            image_pack: ImagePack::default(),
        }
    }

    pub(crate) const fn format(&self) -> wgpu::TextureFormat {
        self.quad.format
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
        let Some((aw, ah)) = self.atlas_dims(atlas_id) else {
            return;
        };
        let Some(bind_group) = self.atlas_bind_groups.get(atlas_id).cloned() else {
            return;
        };
        let instances = ctx.encoder.upload(from_ref(&instance));
        let (uniform, offset, _) = ctx.encoder.uniform(&sprite_uniforms(ctx.viewport, aw, ah));
        if self
            .uniforms
            .as_ref()
            .is_none_or(|(buffer, _)| *buffer != uniform)
        {
            let group = uniform_bind_group(ctx.device, &self.uniform_layout, &uniform);
            self.uniforms = Some((uniform, group));
        }
        self.encode_pass(ctx, &instances, &bind_group, offset);
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
    ) -> Option<SpriteInstance> {
        let SpritePaint {
            opacity: opacity_scale,
            space,
            transform,
        } = paint;
        let (aw, ah) = self.atlas_dims(atlas_id)?;
        let (r, dest) = draw.source.fitted((aw, ah), draw.destination)?;
        let uv_rect = [
            r.origin.x / aw,
            r.origin.y / ah,
            (r.origin.x + r.width) / aw,
            (r.origin.y + r.height) / ah,
        ];
        let opacity = (f32::from(draw.opacity) / 10_000.0 * opacity_scale).clamp(0.0, 1.0);
        Some(SpriteInstance {
            dest: [dest.origin.x, dest.origin.y, dest.width, dest.height],
            uv_rect,
            color: space.rgba(draw.color),
            params: [opacity, sprite_kind_f32(draw.kind), draw.source.phase, 0.0],
            xform: affine_rows(transform),
        })
    }

    fn atlas_dims(&self, atlas_id: usize) -> Option<(f32, f32)> {
        let entry = self.atlases.get(atlas_id)?;
        #[allow(
            clippy::cast_precision_loss,
            reason = "pixel and char-count magnitudes are bounded; sub-unit precision is irrelevant at render scale"
        )]
        Some((entry.width as f32, entry.height as f32))
    }

    fn encode_pass(
        &self,
        ctx: &mut PaintCtx<'_>,
        instances: &UploadedRange,
        atlas_bg: &wgpu::BindGroup,
        uniform_offset: u32,
    ) {
        let (_, uniform_bg) = self.uniforms.as_ref().expect("sprite uniforms initialized");
        let Some(pass) = clipped_pass(ctx, "sigil-renderer sprite pass") else {
            return;
        };
        self.quad.draw(
            pass,
            &[uniform_bg, atlas_bg],
            &[uniform_offset],
            instances.buffer.slice(instances.bytes.clone()),
            1,
        );
    }
}

const fn sprite_uniforms(viewport: (f32, f32), atlas_w: f32, atlas_h: f32) -> SpriteUniforms {
    SpriteUniforms {
        viewport: [viewport.0.max(1.0), viewport.1.max(1.0)],
        atlas_size: [atlas_w, atlas_h],
    }
}

const fn sprite_kind_f32(kind: SpriteKind) -> f32 {
    match kind {
        SpriteKind::Mono => 0.0,
        SpriteKind::Subpixel => 1.0,
        SpriteKind::Poly => 2.0,
    }
}

const _: () = assert!(core::mem::size_of::<SpriteInstance>() == 96);
const _: () = assert!(core::mem::size_of::<SpriteUniforms>() == 16);

const SPRITE_SHADER: &str = include_str!("sprite_paint.wgsl");

#[cfg(test)]
mod tests;
