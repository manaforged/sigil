#![allow(
    clippy::redundant_pub_crate,
    dead_code,
    clippy::indexing_slicing,
    clippy::cast_precision_loss,
    clippy::imprecise_flops
)]
//! Atlas sprite paint pipeline — Mono / Subpixel / Poly three-way split.
//!
//! `SpritePipeline` owns a fixed-size atlas registry keyed by `u32` id.
//! Callers register atlas textures via [`SpritePipeline::register_atlas`]
//! and reference them from [`crate::DrawSprite`] commands via the returned id.
//!
//! # Built-in showcase atlas
//!
//! [`BUILTIN_SPRITE_ATLAS_ID`] is a sentinel id used by `scenes::sprite()`.
//! When `paint_sprite` encounters this sentinel and no atlas has been uploaded
//! yet, it calls an internal helper to generate and upload the 600x200 RGBA8
//! showcase texture (Mono disk | Subpixel RGB bars | Poly red-to-green
//! gradient). The resolved slot is cached in `builtin_atlas_slot`.

use bytemuck::{Pod, Zeroable};

mod atlas;
mod builder;
mod icon;

use crate::scene_command::{DrawSprite, SpriteKind};
use crate::scene_instance::BlendSpace;
use crate::scene_paint::PaintCtx;
use crate::sprite_atlas::{BUILTIN_ATLAS_HEIGHT, BUILTIN_ATLAS_WIDTH, builtin_atlas_rgba8};

use self::atlas::{AtlasDesc, AtlasEntry, MAX_ATLASES, push_atlas, valid_atlas_desc};
use self::builder::{
    UNIT_QUAD_VERTEX_COUNT, atlas_layout, instance_buffer, pipeline, shader, uniform_bind_group,
    uniform_buffer, uniform_layout, unit_quad,
};
use self::icon::is_builtin_icon_atlas;

/// Sentinel atlas id for the built-in 600x200 showcase atlas used by
/// `scenes::sprite()`. The pipeline auto-registers this atlas on first use.
pub const BUILTIN_SPRITE_ATLAS_ID: u32 = u32::MAX - 1;
const BUILTIN_ICON_ATLAS_COUNT: usize = 5;

/// Per-instance GPU record for one sprite draw.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct SpriteInstance {
    /// `[x, y, w, h]` destination in logical pixels.
    dest: [f32; 4],
    /// `[u0, v0, u1, v1]` atlas UV rect in normalised [0, 1] space.
    uv_rect: [f32; 4],
    /// `[r, g, b, a]` linear tint in [0, 1].
    color: [f32; 4],
    /// `[opacity_01, sprite_kind_f32, phase, 0]`.
    params: [f32; 4],
}

/// Extended uniforms that include atlas size for subpixel dx computation.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct SpriteUniforms {
    /// Logical viewport in pixels.
    viewport: [f32; 2],
    /// Atlas texture dimensions in texels.
    atlas_size: [f32; 2],
}

/// Wgpu resources for rendering atlas sprites across three sprite kinds.
pub(crate) struct SpritePipeline {
    format: wgpu::TextureFormat,
    pipeline: wgpu::RenderPipeline,
    uniform_layout: wgpu::BindGroupLayout,
    atlas_layout: wgpu::BindGroupLayout,
    unit_quad: wgpu::Buffer,
    atlases: Vec<AtlasEntry>,
    atlas_bind_groups: Vec<wgpu::BindGroup>,
    sampler: wgpu::Sampler,
    /// Resolved `atlases` index for `BUILTIN_SPRITE_ATLAS_ID`, or `None`
    /// until the first call that needs it.
    builtin_atlas_slot: Option<usize>,
    builtin_icon_slots: [Option<usize>; BUILTIN_ICON_ATLAS_COUNT],
    /// `ImageSource::id` → registered atlas id, so a raster image uploads
    /// its texture exactly once however many frames or nodes draw it.
    image_slots: std::collections::HashMap<u64, u32>,
}

impl SpritePipeline {
    pub(crate) fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = shader(device);
        let uniform_layout = uniform_layout(device);
        let atlas_layout = atlas_layout(device);
        let pipeline = pipeline(device, format, &shader, &uniform_layout, &atlas_layout);
        let unit_quad = unit_quad(device);
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("sigil-renderer sprite sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        Self {
            format,
            pipeline,
            uniform_layout,
            atlas_layout,
            unit_quad,
            atlases: Vec::new(),
            atlas_bind_groups: Vec::new(),
            sampler,
            builtin_atlas_slot: None,
            builtin_icon_slots: [None; BUILTIN_ICON_ATLAS_COUNT],
            image_slots: std::collections::HashMap::new(),
        }
    }

    /// Resolve a raster image to a registered atlas id, uploading its
    /// pixels exactly once per content id. Returns `u32::MAX` when the
    /// registry is full or the pixels fail atlas validation (the sprite
    /// draw then no-ops, same as any invalid atlas id).
    pub(crate) fn register_image(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        source: &sigil_kernel::ImageSource,
    ) -> u32 {
        if let Some(&id) = self.image_slots.get(&source.id()) {
            return id;
        }
        let id = self.register_atlas(
            device,
            queue,
            &AtlasDesc {
                bytes: source.rgba(),
                width: source.width(),
                height: source.height(),
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
            },
        );
        self.image_slots.insert(source.id(), id);
        id
    }

    pub(crate) const fn format(&self) -> wgpu::TextureFormat {
        self.format
    }

    /// Register an atlas texture and return its id.
    ///
    /// Returns `u32::MAX` if the registry is full (`MAX_ATLASES` entries) or
    /// if the descriptor fails atlas validation.
    pub(crate) fn register_atlas(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        desc: &AtlasDesc<'_>,
    ) -> u32 {
        let AtlasDesc {
            bytes,
            width,
            height,
            format,
        } = desc;
        let (bytes, width, height, format) = (*bytes, *width, *height, *format);
        if self.atlases.len() >= MAX_ATLASES || !valid_atlas_desc(bytes, width, height, format) {
            return u32::MAX;
        }
        let id = u32::try_from(self.atlases.len()).unwrap_or(u32::MAX);
        push_atlas(
            device,
            queue,
            &self.atlas_layout,
            &self.sampler,
            &mut self.atlases,
            &mut self.atlas_bind_groups,
            bytes,
            width,
            height,
            format,
        );
        id
    }

    /// Draw a single sprite command.
    ///
    /// If `draw.source.atlas == BUILTIN_SPRITE_ATLAS_ID`, the built-in
    /// 600x200 showcase atlas is uploaded on first call and cached.
    pub(crate) fn paint_sprite(
        &mut self,
        ctx: &mut PaintCtx<'_>,
        draw: &DrawSprite,
        opacity_scale: f32,
        space: BlendSpace,
    ) {
        let Some(atlas_id) = self.resolve_atlas(ctx.device, ctx.queue, draw.source.atlas) else {
            return;
        };
        let Some(instance) = self.build_instance(atlas_id, draw, opacity_scale, space) else {
            return;
        };
        let Some((aw, ah)) = self.atlas_dims(atlas_id) else {
            return;
        };
        let Some(bind_group) = self.atlas_bind_groups.get(atlas_id).cloned() else {
            return;
        };
        let instance_buf = instance_buffer(ctx.device, &instance);
        let uniform_buf = uniform_buffer(ctx.device, &sprite_uniforms(ctx.viewport, aw, ah));
        let uniform_bg = uniform_bind_group(ctx.device, &self.uniform_layout, &uniform_buf);
        self.encode_pass(ctx, &uniform_bg, &instance_buf, &bind_group);
    }

    /// Translate a raw sentinel id to a concrete `atlases` index, uploading
    /// the built-in atlas on demand when the sentinel is seen for the first time.
    fn resolve_atlas(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        raw_id: u32,
    ) -> Option<usize> {
        if is_builtin_icon_atlas(raw_id) {
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
        let bytes = builtin_atlas_rgba8();
        push_atlas(
            device,
            queue,
            &self.atlas_layout,
            &self.sampler,
            &mut self.atlases,
            &mut self.atlas_bind_groups,
            &bytes,
            BUILTIN_ATLAS_WIDTH,
            BUILTIN_ATLAS_HEIGHT,
            wgpu::TextureFormat::Rgba8UnormSrgb,
        );
        self.builtin_atlas_slot = Some(slot);
        Some(slot)
    }

    fn build_instance(
        &self,
        atlas_id: usize,
        draw: &DrawSprite,
        opacity_scale: f32,
        space: BlendSpace,
    ) -> Option<SpriteInstance> {
        let (aw, ah) = self.atlas_dims(atlas_id)?;
        let r = &draw.source.region;
        let uv_rect = [
            r.origin.x / aw,
            r.origin.y / ah,
            (r.origin.x + r.width) / aw,
            (r.origin.y + r.height) / ah,
        ];
        let opacity = (f32::from(draw.opacity) / 10_000.0 * opacity_scale).clamp(0.0, 1.0);
        let dest = &draw.destination;
        Some(SpriteInstance {
            dest: [dest.origin.x, dest.origin.y, dest.width, dest.height],
            uv_rect,
            color: space.rgba(draw.color),
            params: [opacity, sprite_kind_f32(draw.kind), draw.source.phase, 0.0],
        })
    }

    fn atlas_dims(&self, atlas_id: usize) -> Option<(f32, f32)> {
        let entry = self.atlases.get(atlas_id)?;
        #[allow(clippy::cast_precision_loss)]
        Some((entry.width as f32, entry.height as f32))
    }

    fn encode_pass(
        &self,
        ctx: &mut PaintCtx<'_>,
        uniform_bg: &wgpu::BindGroup,
        instance_buf: &wgpu::Buffer,
        atlas_bg: &wgpu::BindGroup,
    ) {
        let mut pass = ctx.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("sigil-renderer sprite pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: ctx.view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, uniform_bg, &[]);
        pass.set_bind_group(1, atlas_bg, &[]);
        pass.set_vertex_buffer(0, self.unit_quad.slice(..));
        pass.set_vertex_buffer(1, instance_buf.slice(..));
        pass.draw(0..UNIT_QUAD_VERTEX_COUNT, 0..1);
    }
}

// ---- GPU helpers ------------------------------------------------------------

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

// Verify GPU struct sizes match what bytemuck will produce.
const _: () = assert!(core::mem::size_of::<SpriteInstance>() == 64);
const _: () = assert!(core::mem::size_of::<SpriteUniforms>() == 16);

const SPRITE_SHADER: &str = include_str!("sprite_paint.wgsl");

// ---- bench / test helpers ---------------------------------------------------

/// Construct a fresh [`SpritePipeline`] against `device` using the sRGB format.
///
/// Exposed for `benches/` and integration tests that need direct pipeline access.
/// Production callers go through [`WgpuRenderer`][crate::renderer::WgpuRenderer].
#[doc(hidden)]
#[cfg(any(test, feature = "bench"))]
pub(crate) fn sprite_pipeline_new(device: &wgpu::Device) -> SpritePipeline {
    SpritePipeline::new(device, wgpu::TextureFormat::Rgba8UnormSrgb)
}

/// Return the maximum number of atlases the registry can hold.
///
/// Used by property tests to drive boundary coverage.
#[doc(hidden)]
#[cfg(any(test, feature = "bench"))]
pub(crate) const fn max_atlases_for_test() -> usize {
    MAX_ATLASES
}

#[cfg(any(test, feature = "bench"))]
impl SpritePipeline {
    /// Register a raw RGBA8 atlas for tests / benches.
    ///
    /// Behaves identically to the crate-internal [`Self::register_atlas`]
    /// but is pub so integration test crates can call it directly.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn register_atlas_for_test(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bytes: &[u8],
        width: u32,
        height: u32,
    ) -> u32 {
        self.register_atlas(
            device,
            queue,
            &AtlasDesc {
                bytes,
                width,
                height,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
            },
        )
    }
}

#[cfg(test)]
mod tests;
