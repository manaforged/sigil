use std::mem::replace;

use crate::profile::count;
use crate::renderer::decoration_paint::DecorationInstance;
use crate::renderer::frame_upload::{UploadBuffers, UploadedRange};
use crate::renderer::layer_composite::LayerInstance;
use crate::renderer::raster_paint::RasterInstance;
use crate::renderer::scene_command::SceneCommand;
use crate::renderer::scene_instance::{
    FillInstance, GpuStop, MAX_GRADIENT_STOPS, RectInstance, SceneUniforms, ShadowInstance,
};
use crate::renderer::slug::SLUG_INSTANCE_BYTES;
use crate::renderer::sprite_paint::SpriteInstance;

pub(crate) const MAX_PASSES_PER_SUBMISSION: u32 = 1_024;

pub(crate) struct SplitEncoder {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pass: Option<wgpu::RenderPass<'static>>,
    target: Option<wgpu::TextureView>,
    encoder: wgpu::CommandEncoder,
    passes: u32,
    uploads: UploadBuffers,
    uniform_alignment: u64,
    storage_alignment: u64,
}

impl SplitEncoder {
    pub(crate) fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: wgpu::CommandEncoder,
    ) -> Self {
        Self {
            device: device.clone(),
            queue: queue.clone(),
            pass: None,
            target: None,
            encoder,
            passes: 0,
            uploads: UploadBuffers::default(),
            uniform_alignment: u64::from(device.limits().min_uniform_buffer_offset_alignment),
            storage_alignment: u64::from(device.limits().min_storage_buffer_offset_alignment),
        }
    }

    pub(crate) fn reuse_uploads(&mut self, uploads: &mut Option<UploadBuffers>) {
        if let Some(uploads) = uploads.take() {
            self.uploads = uploads;
        }
    }

    pub(crate) fn reserve_draws(&mut self, commands: &[SceneCommand]) {
        let mut vertices = 0;
        let mut uniforms = size_of::<SceneUniforms>() as u64;
        let mut storage = 0;
        for command in commands {
            match command {
                SceneCommand::Rect(_) => vertices += size_of::<RectInstance>() as u64,
                SceneCommand::Gradient(draw) => {
                    let stops = draw.gradient.stops.len().clamp(1, MAX_GRADIENT_STOPS);
                    storage += (stops * size_of::<GpuStop>()) as u64 + self.storage_alignment;
                    vertices += size_of::<FillInstance>() as u64;
                }
                SceneCommand::Pattern(_) => {
                    storage += size_of::<GpuStop>() as u64 + self.storage_alignment;
                    vertices += size_of::<FillInstance>() as u64;
                }
                SceneCommand::DrawShadow { .. } => vertices += size_of::<ShadowInstance>() as u64,
                SceneCommand::Decoration(_) => vertices += size_of::<DecorationInstance>() as u64,
                SceneCommand::Sprite(_) | SceneCommand::Image(_) => {
                    vertices += size_of::<SpriteInstance>() as u64;
                    uniforms += self.uniform_alignment;
                }
                SceneCommand::GlyphRun(draw) => {
                    vertices += (draw.run.positioned_glyphs().len()
                        * size_of::<RasterInstance>().max(SLUG_INSTANCE_BYTES))
                        as u64;
                    uniforms += self.uniform_alignment;
                }
                SceneCommand::PushLayer(_) => {
                    vertices += size_of::<LayerInstance>() as u64;
                    uniforms += 3 * self.uniform_alignment;
                }
                _ => {}
            }
        }
        self.uploads.vertices.reserve(vertices);
        self.uploads.uniforms.reserve(uniforms);
        self.uploads.storage.reserve(storage);
    }

    pub(crate) fn upload<T: bytemuck::Pod>(&mut self, values: &[T]) -> UploadedRange {
        let (buffer, bytes, allocated_bytes) = self.uploads.vertices.write(
            &self.device,
            &self.queue,
            bytemuck::cast_slice(values),
            wgpu::COPY_BUFFER_ALIGNMENT,
        );
        UploadedRange {
            buffer,
            bytes,
            count: u32::try_from(values.len()).expect("draw count exceeds u32"),
            allocated_bytes,
        }
    }

    pub(crate) fn uniform<T: bytemuck::Pod>(&mut self, value: &T) -> (wgpu::Buffer, u32, u64) {
        let (buffer, bytes, allocated_bytes) = self.uploads.uniforms.write(
            &self.device,
            &self.queue,
            bytemuck::bytes_of(value),
            self.uniform_alignment,
        );
        (
            buffer,
            u32::try_from(bytes.start).expect("uniform offset exceeds u32"),
            allocated_bytes,
        )
    }

    pub(crate) fn storage<T: bytemuck::Pod>(&mut self, values: &[T]) -> UploadedRange {
        let (buffer, bytes, allocated_bytes) = self.uploads.storage.write(
            &self.device,
            &self.queue,
            bytemuck::cast_slice(values),
            self.storage_alignment,
        );
        UploadedRange {
            buffer,
            bytes,
            count: u32::try_from(values.len()).expect("stop count exceeds u32"),
            allocated_bytes,
        }
    }

    pub(crate) fn begin_render_pass(
        &mut self,
        desc: &wgpu::RenderPassDescriptor<'_>,
    ) -> &mut wgpu::RenderPass<'static> {
        let target = reusable_target(desc);
        let load = desc
            .color_attachments
            .first()
            .and_then(Option::as_ref)
            .is_some_and(|attachment| attachment.ops.load == wgpu::LoadOp::Load);
        if load
            && let Some(target) = target
            && Some(target) == self.target.as_ref()
        {
            count("gpu.pass_reuses");
            let size = target.texture().size();
            let pass = self.pass.as_mut().expect("active render pass");
            pass.set_scissor_rect(0, 0, size.width, size.height);
            return pass;
        }
        drop(self.pass.take());
        self.passes += 1;
        count("gpu.render_passes");
        if self.passes > MAX_PASSES_PER_SUBMISSION {
            let fresh = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("sigil-renderer split submission"),
                });
            let full = replace(&mut self.encoder, fresh);
            self.flush();
            self.queue.submit(core::iter::once(full.finish()));
            count("gpu.split_submissions");
            self.passes = 1;
        }
        self.target = target.cloned();
        self.pass = Some(self.encoder.begin_render_pass(desc).forget_lifetime());
        self.pass.as_mut().expect("render pass just started")
    }

    pub(crate) fn submit(mut self) -> UploadBuffers {
        drop(self.pass.take());
        self.flush();
        self.queue.submit(core::iter::once(self.encoder.finish()));
        self.uploads.reset();
        self.uploads
    }

    pub(crate) fn into_inner(mut self) -> wgpu::CommandEncoder {
        drop(self.pass.take());
        self.flush();
        self.encoder
    }

    fn flush(&mut self) {
        self.uploads.vertices.flush(&self.queue);
        self.uploads.uniforms.flush(&self.queue);
        self.uploads.storage.flush(&self.queue);
    }
}

fn reusable_target<'a>(desc: &wgpu::RenderPassDescriptor<'a>) -> Option<&'a wgpu::TextureView> {
    if desc.color_attachments.len() != 1
        || desc.depth_stencil_attachment.is_some()
        || desc.timestamp_writes.is_some()
        || desc.occlusion_query_set.is_some()
        || desc.multiview_mask.is_some()
    {
        return None;
    }
    let attachment = desc.color_attachments.first()?.as_ref()?;
    let texture = attachment.view.texture();
    (attachment.resolve_target.is_none()
        && attachment.depth_slice.is_none()
        && attachment.ops.store == wgpu::StoreOp::Store
        && texture.mip_level_count() == 1
        && texture.depth_or_array_layers() == 1)
        .then_some(attachment.view)
}
