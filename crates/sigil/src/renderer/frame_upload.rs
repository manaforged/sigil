use std::ops::Range;

use bytemuck::Pod;

use crate::profile::{count, count_add};
use crate::renderer::frame_stats::FrameStats;
use crate::renderer::scene_command::SceneCommand;
use crate::renderer::scene_instance::{GpuStop, MAX_GRADIENT_STOPS, SceneUniforms};
use crate::renderer::split_encoder::SplitEncoder;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct UploadCapacities {
    rects: usize,
    fills: usize,
    stops: usize,
    shadows: usize,
    decorations: usize,
    glyphs: usize,
}

impl UploadCapacities {
    pub(crate) fn from_commands<'a>(commands: impl IntoIterator<Item = &'a SceneCommand>) -> Self {
        let mut out = Self::default();
        for command in commands {
            match command {
                SceneCommand::Rect(_) => out.rects += 1,
                SceneCommand::Gradient(draw) => {
                    out.fills += 1;
                    out.stops += draw.gradient.stops.len().clamp(1, MAX_GRADIENT_STOPS);
                }
                SceneCommand::Pattern(_) => out.fills += 1,
                SceneCommand::DrawShadow { .. } => out.shadows += 1,
                SceneCommand::Decoration(_) => out.decorations += 1,
                SceneCommand::GlyphRun(draw) => {
                    out.glyphs += draw.run.positioned_glyphs().len();
                }
                _ => {}
            }
        }
        out
    }

    const fn has_primitives(self) -> bool {
        self.rects > 0 || self.fills > 0 || self.shadows > 0 || self.decorations > 0
    }
}

pub(crate) struct UploadedRange {
    pub(crate) buffer: wgpu::Buffer,
    pub(crate) bytes: Range<u64>,
    pub(crate) count: u32,
    pub(crate) allocated_bytes: u64,
}

pub(crate) struct FrameUploads {
    uniform: Option<(wgpu::Buffer, u32)>,
}

impl FrameUploads {
    pub(crate) fn new(
        encoder: &mut SplitEncoder,
        viewport: (f32, f32),
        capacity: UploadCapacities,
        stats: &mut FrameStats,
    ) -> Self {
        let uniform = capacity
            .has_primitives()
            .then(|| make_uniform(encoder, viewport, stats));
        Self { uniform }
    }

    pub(crate) fn uniform(&self) -> Option<wgpu::BufferBinding<'_>> {
        self.uniform
            .as_ref()
            .map(|(buffer, offset)| wgpu::BufferBinding {
                buffer,
                offset: u64::from(*offset),
                size: wgpu::BufferSize::new(size_of::<SceneUniforms>() as u64),
            })
    }

    pub(crate) fn upload<T: Pod>(
        encoder: &mut SplitEncoder,
        values: &[T],
        stats: &mut FrameStats,
    ) -> Option<UploadedRange> {
        if values.is_empty() {
            return None;
        }
        let uploaded = encoder.upload(values);
        if uploaded.allocated_bytes > 0 {
            stats.record_primitive_buffer(uploaded.allocated_bytes);
        }
        stats.record_primitive_write(uploaded.bytes.end - uploaded.bytes.start);
        Some(uploaded)
    }

    pub(crate) fn upload_stops(
        encoder: &mut SplitEncoder,
        values: &[GpuStop],
        stats: &mut FrameStats,
    ) -> UploadedRange {
        let values = if values.is_empty() {
            core::slice::from_ref(&EMPTY_STOP)
        } else {
            values
        };
        let uploaded = encoder.storage(values);
        if uploaded.allocated_bytes > 0 {
            stats.record_primitive_buffer(uploaded.allocated_bytes);
        }
        stats.record_primitive_write(uploaded.bytes.end - uploaded.bytes.start);
        uploaded
    }
}

pub(crate) struct UploadBuffer {
    label: &'static str,
    usage: wgpu::BufferUsages,
    buffer: Option<wgpu::Buffer>,
    capacity: u64,
    written: u64,
    pending: Vec<u8>,
    pending_offset: u64,
}

impl UploadBuffer {
    pub(crate) fn new_bytes(
        label: &'static str,
        capacity: usize,
        stride: usize,
        usage: wgpu::BufferUsages,
    ) -> Self {
        let bytes = byte_len(capacity, stride);
        Self {
            label,
            usage,
            buffer: None,
            capacity: bytes,
            written: 0,
            pending: Vec::new(),
            pending_offset: 0,
        }
    }

    pub(crate) fn reserve(&mut self, bytes: u64) {
        self.capacity = self.capacity.max(bytes);
    }

    pub(crate) fn write(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bytes: &[u8],
        alignment: u64,
    ) -> (wgpu::Buffer, Range<u64>, u64) {
        let mut start = self.written.next_multiple_of(alignment);
        let end = start + bytes.len() as u64;
        let mut allocated_bytes = 0;
        if self
            .buffer
            .as_ref()
            .is_none_or(|buffer| end > buffer.size())
        {
            self.flush(queue);
            self.capacity = self
                .capacity
                .max(end)
                .next_multiple_of(wgpu::COPY_BUFFER_ALIGNMENT);
            self.buffer = Some(device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(self.label),
                size: self.capacity,
                usage: self.usage | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));
            count("alloc.upload.buffers");
            count_add("alloc.upload.bytes", self.capacity);
            allocated_bytes = self.capacity;
            start = 0;
        }
        let buffer = self.buffer.as_ref().expect("upload buffer initialized");
        if self.pending.is_empty() {
            self.pending_offset = start;
        }
        let padding = usize::try_from(start - self.pending_offset).expect("upload exceeds usize");
        self.pending.resize(padding, 0);
        self.pending.extend_from_slice(bytes);
        self.written = start + bytes.len() as u64;
        (buffer.clone(), start..self.written, allocated_bytes)
    }

    pub(crate) fn flush(&mut self, queue: &wgpu::Queue) {
        if self.pending.is_empty() {
            return;
        }
        let buffer = self.buffer.as_ref().expect("upload buffer initialized");
        queue.write_buffer(buffer, self.pending_offset, &self.pending);
        count("gpu.shared_upload_writes");
        count_add("gpu.shared_upload_bytes", self.pending.len() as u64);
        self.pending.clear();
    }
}

const EMPTY_STOP: GpuStop = GpuStop {
    offset_bits: 0,
    rgba01: 0,
    rgba23: 0,
    _pad: 0,
};

fn make_uniform(
    encoder: &mut SplitEncoder,
    viewport: (f32, f32),
    stats: &mut FrameStats,
) -> (wgpu::Buffer, u32) {
    let uniform = SceneUniforms {
        viewport: [viewport.0.max(1.0), viewport.1.max(1.0)],
        _pad: [0.0, 0.0],
    };
    let (buffer, offset, allocated_bytes) = encoder.uniform(&uniform);
    if allocated_bytes > 0 {
        stats.record_primitive_buffer(allocated_bytes);
    }
    stats.record_primitive_write(size_of::<SceneUniforms>() as u64);
    (buffer, offset)
}

fn byte_len(count: usize, stride: usize) -> u64 {
    u64::try_from(count.saturating_mul(stride)).unwrap_or(u64::MAX)
}

#[cfg(test)]
#[path = "frame_upload_tests.rs"]
mod tests;
