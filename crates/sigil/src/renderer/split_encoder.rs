mod timing;

use std::mem::replace;

use crate::profile::count;
use crate::renderer::decoration_paint::DecorationInstance;
use crate::renderer::frame_upload::{UploadBuffer, UploadBuffers, UploadedRange};
use crate::renderer::layer_composite::LayerInstance;
use crate::renderer::raster_paint::RasterInstance;
use crate::renderer::renderer::record_clear;
use crate::renderer::scene_command::SceneCommand;
use crate::renderer::scene_instance::{
    FillInstance, GpuStop, MAX_GRADIENT_STOPS, RectInstance, SceneUniforms, ShadowInstance,
};
use crate::renderer::slug::SLUG_INSTANCE_BYTES;
use crate::renderer::sprite_paint::SpriteInstance;
use crate::style::Color;

pub(crate) use timing::GpuTimer;

pub(crate) const MAX_PASSES_PER_SUBMISSION: u32 = 1_024;

pub(crate) struct SplitEncoder {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pass: Option<wgpu::RenderPass<'static>>,
    target: Option<wgpu::TextureView>,
    pending_clear: Option<(wgpu::TextureView, Color)>,
    encoder: wgpu::CommandEncoder,
    passes: u32,
    uploads: UploadBuffers,
    uniform_alignment: u64,
    storage_alignment: u64,
    timer: Option<GpuTimer>,
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
            pending_clear: None,
            encoder,
            passes: 0,
            uploads: UploadBuffers::default(),
            uniform_alignment: u64::from(device.limits().min_uniform_buffer_offset_alignment),
            storage_alignment: u64::from(device.limits().min_storage_buffer_offset_alignment),
            timer: None,
        }
    }

    pub(crate) fn time_passes(&mut self, timer: &mut Option<GpuTimer>) {
        if !crate::profile::enabled() || !GpuTimer::supported(&self.device) {
            return;
        }
        let mut owned = timer.take().unwrap_or_default();
        if owned.begin(&self.device, &self.queue) {
            owned.mark_start(&self.device, &mut self.encoder);
            self.timer = Some(owned);
        } else {
            *timer = Some(owned);
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
                SceneCommand::PushLayer(_) | SceneCommand::SaveLayer { .. } => {
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
        let alignment = wgpu::COPY_BUFFER_ALIGNMENT;
        write_range(
            &mut self.uploads.vertices,
            &self.queue,
            &self.device,
            values,
            alignment,
        )
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
        let alignment = self.storage_alignment;
        write_range(
            &mut self.uploads.storage,
            &self.queue,
            &self.device,
            values,
            alignment,
        )
    }

    fn end_pass(&mut self) {
        self.flush_clear();
        drop(self.pass.take());
        self.target = None;
    }

    pub(crate) fn clear(&mut self, view: &wgpu::TextureView, color: Color) {
        self.end_pass();
        self.pending_clear = Some((view.clone(), color));
    }

    fn flush_clear(&mut self) {
        if let Some((view, color)) = self.pending_clear.take() {
            drop(self.pass.take());
            self.target = None;
            self.passes += 1;
            record_clear(&mut self.encoder, &view, color);
        }
    }

    fn take_fold(&mut self, desc: &wgpu::RenderPassDescriptor<'_>) -> Option<Color> {
        let first = desc.color_attachments.first().and_then(Option::as_ref);
        let folds = matches!(
            (&self.pending_clear, first),
            (Some((view, _)), Some(attachment)) if *view == *attachment.view
        );
        if folds {
            return self.pending_clear.take().map(|(_, color)| color);
        }
        self.flush_clear();
        None
    }

    pub(crate) fn begin_render_pass(
        &mut self,
        label: &'static str,
        desc: &wgpu::RenderPassDescriptor<'_>,
    ) -> &mut wgpu::RenderPass<'static> {
        let fold = self.take_fold(desc);
        let target = reusable_target(desc);
        let load = desc
            .color_attachments
            .first()
            .and_then(Option::as_ref)
            .is_some_and(|attachment| attachment.ops.load == wgpu::LoadOp::Load);
        if load
            && fold.is_none()
            && let Some(target) = target
            && Some(target) == self.target.as_ref()
        {
            count(crate::profile::Counter::GpuPassReuses);
            let size = target.texture().size();
            let pass = self.pass.as_mut().expect("active render pass");
            pass.set_scissor_rect(0, 0, size.width, size.height);
            return pass;
        }
        drop(self.pass.take());
        self.passes += 1;
        count(crate::profile::Counter::GpuRenderPasses);
        if self.passes > MAX_PASSES_PER_SUBMISSION {
            let fresh = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("sigil-renderer split submission"),
                });
            let full = replace(&mut self.encoder, fresh);
            self.flush();
            self.queue.submit(core::iter::once(full.finish()));
            count(crate::profile::Counter::GpuSplitSubmissions);
            self.passes = 1;
        }
        self.target = target.cloned();
        let timed = self.timer.as_mut().and_then(|timer| timer.next(label));
        let labelled = timed_desc(desc, label, timed.as_ref());
        self.pass = Some(open_pass(&mut self.encoder, &labelled, fold));
        self.pass.as_mut().expect("render pass just started")
    }

    pub(crate) fn copy_texture(&mut self, source: &wgpu::Texture, destination: &wgpu::Texture) {
        self.end_pass();
        self.encoder.copy_texture_to_texture(
            source.as_image_copy(),
            destination.as_image_copy(),
            source.size(),
        );
    }

    pub(crate) fn submit(mut self) -> (UploadBuffers, Option<GpuTimer>) {
        self.end_pass();
        self.flush();
        if let Some(timer) = self.timer.as_mut() {
            timer.terminate(&self.device, &mut self.encoder);
        }
        self.queue.submit(core::iter::once(self.encoder.finish()));
        if let Some(timer) = self.timer.as_mut() {
            let mut resolve = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("sigil-renderer timestamp resolve"),
                });
            timer.resolve(&self.device, &mut resolve);
            self.queue.submit(core::iter::once(resolve.finish()));
            timer.submitted();
        }
        self.uploads.reset();
        (self.uploads, self.timer)
    }

    pub(crate) fn into_inner(mut self) -> wgpu::CommandEncoder {
        self.end_pass();
        self.flush();
        self.encoder
    }

    fn flush(&mut self) {
        self.uploads.vertices.flush(&self.queue);
        self.uploads.uniforms.flush(&self.queue);
        self.uploads.storage.flush(&self.queue);
    }
}

fn write_range<T: bytemuck::Pod>(
    buffer: &mut UploadBuffer,
    queue: &wgpu::Queue,
    device: &wgpu::Device,
    values: &[T],
    alignment: u64,
) -> UploadedRange {
    let (buffer, bytes, allocated_bytes) =
        buffer.write(device, queue, bytemuck::cast_slice(values), alignment);
    UploadedRange {
        buffer,
        bytes,
        count: u32::try_from(values.len()).expect("upload count exceeds u32"),
        allocated_bytes,
    }
}

fn timed_desc<'a>(
    desc: &wgpu::RenderPassDescriptor<'a>,
    label: &'static str,
    timed: Option<&'a (wgpu::QuerySet, u32)>,
) -> wgpu::RenderPassDescriptor<'a> {
    let mut labelled = desc.clone();
    labelled.label = Some(label);
    labelled.timestamp_writes = timed.map(|(queries, index)| wgpu::RenderPassTimestampWrites {
        query_set: queries,
        beginning_of_pass_write_index: Some(*index),
        end_of_pass_write_index: None,
    });
    labelled
}

fn open_pass(
    encoder: &mut wgpu::CommandEncoder,
    desc: &wgpu::RenderPassDescriptor<'_>,
    fold: Option<Color>,
) -> wgpu::RenderPass<'static> {
    let Some(color) = fold else {
        return encoder.begin_render_pass(desc).forget_lifetime();
    };
    let mut attachments = desc.color_attachments.to_vec();
    if let Some(Some(first)) = attachments.first_mut()
        && first.ops.load == wgpu::LoadOp::Load
    {
        first.ops.load = wgpu::LoadOp::Clear(clear_value(color));
    }
    let mut cleared = desc.clone();
    cleared.color_attachments = &attachments;
    encoder.begin_render_pass(&cleared).forget_lifetime()
}

pub(crate) fn clear_value(color: Color) -> wgpu::Color {
    wgpu::Color {
        r: f64::from(color.r),
        g: f64::from(color.g),
        b: f64::from(color.b),
        a: f64::from(color.a),
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
