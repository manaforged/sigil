use std::ops::Range;

use bytemuck::Pod;

use crate::profile::{count, count_add};
use crate::renderer::frame_stats::FrameStats;
use crate::renderer::scene_command::SceneCommand;
use crate::renderer::scene_instance::{GpuStop, SceneUniforms};
use crate::renderer::split_encoder::SplitEncoder;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct UploadCapacities {
    has_primitives: bool,
}

impl UploadCapacities {
    pub(crate) fn from_commands<'a>(commands: impl IntoIterator<Item = &'a SceneCommand>) -> Self {
        let has_primitives = commands.into_iter().any(|command| {
            matches!(
                command,
                SceneCommand::Rect(_)
                    | SceneCommand::Gradient(_)
                    | SceneCommand::Pattern(_)
                    | SceneCommand::DrawShadow { .. }
                    | SceneCommand::Decoration(_)
            )
        });
        Self { has_primitives }
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
        let uniform = capacity.has_primitives.then(|| {
            let (buffer, offset, allocated_bytes) = encoder.uniform(&SceneUniforms {
                viewport: [viewport.0.max(1.0), viewport.1.max(1.0)],
                _pad: [0.0, 0.0],
            });
            record(stats, allocated_bytes, size_of::<SceneUniforms>() as u64);
            (buffer, offset)
        });
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
        Some(recorded(encoder.upload(values), stats))
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
        recorded(encoder.storage(values), stats)
    }
}

const fn recorded(uploaded: UploadedRange, stats: &mut FrameStats) -> UploadedRange {
    record(
        stats,
        uploaded.allocated_bytes,
        uploaded.bytes.end - uploaded.bytes.start,
    );
    uploaded
}

const fn record(stats: &mut FrameStats, allocated_bytes: u64, written: u64) {
    if allocated_bytes > 0 {
        stats.record_primitive_buffer(allocated_bytes);
    }
    stats.record_primitive_write(written);
}

pub(crate) struct UploadBuffers {
    pub(crate) vertices: UploadBuffer,
    pub(crate) uniforms: UploadBuffer,
    pub(crate) storage: UploadBuffer,
}

impl Default for UploadBuffers {
    fn default() -> Self {
        Self {
            vertices: UploadBuffer::new("sigil draw instances", wgpu::BufferUsages::VERTEX),
            uniforms: UploadBuffer::new("sigil draw uniforms", wgpu::BufferUsages::UNIFORM),
            storage: UploadBuffer::new("sigil gradient stops", wgpu::BufferUsages::STORAGE),
        }
    }
}

impl UploadBuffers {
    pub(crate) fn reset(&mut self) {
        for buffer in [&mut self.vertices, &mut self.uniforms, &mut self.storage] {
            debug_assert!(buffer.pending.is_empty());
            buffer.written = 0;
            buffer.pending_offset = 0;
        }
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
    const fn new(label: &'static str, usage: wgpu::BufferUsages) -> Self {
        Self {
            label,
            usage,
            buffer: None,
            capacity: 0,
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
            count(crate::profile::Counter::AllocUploadBuffers);
            count_add(crate::profile::Counter::AllocUploadBytes, self.capacity);
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
        count(crate::profile::Counter::GpuSharedUploadWrites);
        count_add(
            crate::profile::Counter::GpuSharedUploadBytes,
            self.pending.len() as u64,
        );
        self.pending.clear();
    }
}

const EMPTY_STOP: GpuStop = GpuStop {
    offset_bits: 0,
    rgba01: 0,
    rgba23: 0,
    _pad: 0,
};

#[cfg(test)]
#[path = "frame_upload_tests.rs"]
mod tests;
