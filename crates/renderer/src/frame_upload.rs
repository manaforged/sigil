//! Per-walk GPU upload storage for ordered primitive batches.

use std::ops::Range;

use bytemuck::Pod;

use crate::decoration_paint::DecorationInstance;
use crate::frame_stats::FrameStats;
use crate::scene_command::SceneCommand;
use crate::scene_instance::{
    FillInstance, GpuStop, MAX_GRADIENT_STOPS, RectInstance, SceneUniforms, ShadowInstance,
};

/// Maximum primitive payloads a command slice can produce.
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
        self.rects > 0
            || self.fills > 0
            || self.shadows > 0
            || self.decorations > 0
            || self.glyphs > 0
    }
}

/// One uploaded section of a frame-owned GPU buffer.
pub(crate) struct UploadedRange {
    pub(crate) buffer: wgpu::Buffer,
    pub(crate) bytes: Range<u64>,
    pub(crate) count: u32,
}

/// Buffers shared by every ordered primitive batch in one command walk.
pub(crate) struct FrameUploads {
    device: wgpu::Device,
    uniform: Option<wgpu::Buffer>,
    rects: UploadBuffer,
    fills: UploadBuffer,
    stops: UploadBuffer,
    shadows: UploadBuffer,
    decorations: UploadBuffer,
    raster_glyphs: UploadBuffer,
    slug_glyphs: UploadBuffer,
}

impl FrameUploads {
    #[allow(clippy::too_many_lines)]
    pub(crate) fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        viewport: (f32, f32),
        capacity: UploadCapacities,
        stats: &mut FrameStats,
    ) -> Self {
        let uniform = capacity
            .has_primitives()
            .then(|| make_uniform(device, queue, viewport, stats));
        Self {
            device: device.clone(),
            uniform,
            rects: UploadBuffer::new::<RectInstance>(
                "sigil frame rect instances",
                capacity.rects,
                wgpu::BufferUsages::VERTEX,
            ),
            fills: UploadBuffer::new::<FillInstance>(
                "sigil frame fill instances",
                capacity.fills,
                wgpu::BufferUsages::VERTEX,
            ),
            stops: UploadBuffer::new::<GpuStop>(
                "sigil frame gradient stops",
                capacity.stops.max(usize::from(capacity.fills > 0)),
                wgpu::BufferUsages::STORAGE,
            ),
            shadows: UploadBuffer::new::<ShadowInstance>(
                "sigil frame shadow instances",
                capacity.shadows,
                wgpu::BufferUsages::VERTEX,
            ),
            decorations: UploadBuffer::new::<DecorationInstance>(
                "sigil frame decoration instances",
                capacity.decorations,
                wgpu::BufferUsages::VERTEX,
            ),
            raster_glyphs: UploadBuffer::new_bytes(
                "sigil frame raster glyph instances",
                capacity.glyphs,
                crate::raster_paint::RASTER_INSTANCE_BYTES,
                wgpu::BufferUsages::VERTEX,
            ),
            slug_glyphs: UploadBuffer::new_bytes(
                "sigil frame slug glyph instances",
                capacity.glyphs,
                crate::slug::SLUG_INSTANCE_BYTES,
                wgpu::BufferUsages::VERTEX,
            ),
        }
    }

    pub(crate) const fn uniform(&self) -> Option<&wgpu::Buffer> {
        self.uniform.as_ref()
    }

    pub(crate) fn upload_rects(
        &mut self,
        queue: &wgpu::Queue,
        values: &[RectInstance],
        stats: &mut FrameStats,
    ) -> Option<UploadedRange> {
        self.rects.upload(&self.device, queue, values, stats)
    }

    pub(crate) fn upload_fills(
        &mut self,
        queue: &wgpu::Queue,
        values: &[FillInstance],
        stats: &mut FrameStats,
    ) -> Option<UploadedRange> {
        self.fills.upload(&self.device, queue, values, stats)
    }

    pub(crate) fn next_stop_offset(&self, pending: usize) -> u32 {
        let written = self.stops.written / core::mem::size_of::<GpuStop>() as u64;
        u32::try_from(written.saturating_add(pending as u64)).unwrap_or(u32::MAX)
    }

    pub(crate) fn upload_stops(
        &mut self,
        queue: &wgpu::Queue,
        values: &[GpuStop],
        stats: &mut FrameStats,
    ) {
        let values = if values.is_empty() {
            core::slice::from_ref(&EMPTY_STOP)
        } else {
            values
        };
        let _ = self.stops.upload(&self.device, queue, values, stats);
    }

    pub(crate) const fn stops(&self) -> Option<&wgpu::Buffer> {
        self.stops.buffer.as_ref()
    }

    pub(crate) fn upload_shadows(
        &mut self,
        queue: &wgpu::Queue,
        values: &[ShadowInstance],
        stats: &mut FrameStats,
    ) -> Option<UploadedRange> {
        self.shadows.upload(&self.device, queue, values, stats)
    }

    pub(crate) fn upload_decorations(
        &mut self,
        queue: &wgpu::Queue,
        values: &[DecorationInstance],
        stats: &mut FrameStats,
    ) -> Option<UploadedRange> {
        self.decorations.upload(&self.device, queue, values, stats)
    }

    pub(crate) fn upload_raster<T: Pod>(
        &mut self,
        queue: &wgpu::Queue,
        values: &[T],
        stats: &mut FrameStats,
    ) -> Option<UploadedRange> {
        self.raster_glyphs
            .upload(&self.device, queue, values, stats)
    }

    pub(crate) fn upload_slug<T: Pod>(
        &mut self,
        queue: &wgpu::Queue,
        values: &[T],
        stats: &mut FrameStats,
    ) -> Option<UploadedRange> {
        self.slug_glyphs.upload(&self.device, queue, values, stats)
    }
}

struct UploadBuffer {
    label: &'static str,
    usage: wgpu::BufferUsages,
    buffer: Option<wgpu::Buffer>,
    capacity: u64,
    written: u64,
}

impl UploadBuffer {
    fn new<T: Pod>(label: &'static str, capacity: usize, usage: wgpu::BufferUsages) -> Self {
        Self::new_bytes(label, capacity, core::mem::size_of::<T>(), usage)
    }

    fn new_bytes(
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
        }
    }

    fn upload<T: Pod>(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        values: &[T],
        stats: &mut FrameStats,
    ) -> Option<UploadedRange> {
        let bytes = bytemuck::cast_slice(values);
        if bytes.is_empty() || self.capacity == 0 {
            return None;
        }
        if self.buffer.is_none() {
            stats.record_primitive_buffer(self.capacity);
            self.buffer = Some(device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(self.label),
                size: self.capacity,
                usage: self.usage | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));
        }
        let buffer = self.buffer.as_ref().expect("upload buffer was initialized");
        let start = self.written;
        let end = start.saturating_add(bytes.len() as u64);
        assert!(
            end <= self.capacity,
            "primitive upload capacity underestimated"
        );
        queue.write_buffer(buffer, start, bytes);
        self.written = end;
        stats.record_primitive_write(bytes.len() as u64);
        Some(UploadedRange {
            buffer: buffer.clone(),
            bytes: start..end,
            count: u32::try_from(values.len()).unwrap_or(u32::MAX),
        })
    }
}

const EMPTY_STOP: GpuStop = GpuStop {
    offset_bits: 0,
    rgba01: 0,
    rgba23: 0,
    _pad: 0,
};

fn make_uniform(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    viewport: (f32, f32),
    stats: &mut FrameStats,
) -> wgpu::Buffer {
    let uniform = SceneUniforms {
        viewport: [viewport.0.max(1.0), viewport.1.max(1.0)],
        _pad: [0.0, 0.0],
    };
    let bytes = bytemuck::bytes_of(&uniform);
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("sigil frame primitive uniform"),
        size: bytes.len() as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(&buffer, 0, bytes);
    stats.record_primitive_buffer(bytes.len() as u64);
    stats.record_primitive_write(bytes.len() as u64);
    buffer
}

fn byte_len(count: usize, stride: usize) -> u64 {
    u64::try_from(count.saturating_mul(stride)).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use sigil_hal::{Point, Rect};
    use sigil_style::Rgba8;

    use super::*;
    use crate::scene_command::{GradientStop, LinearGradient};
    use crate::{CornerRadii, SceneGradientStyle, SceneRectStyle};

    #[test]
    fn capacity_counts_ordered_primitive_payloads() {
        let mut scene = crate::SceneBuilder::new();
        scene.draw_rect(rect(), SceneRectStyle::solid(color()));
        scene.draw_gradient(
            rect(),
            SceneGradientStyle::new(
                LinearGradient::new(
                    Point::new(0.0, 0.0),
                    Point::new(10.0, 0.0),
                    vec![
                        GradientStop::new(0.0, color()),
                        GradientStop::new(1.0, color()),
                    ],
                ),
                CornerRadii::ZERO,
                10_000,
            ),
        );
        let scene = scene.finish();

        assert_eq!(
            UploadCapacities::from_commands(scene.commands.iter()),
            UploadCapacities {
                rects: 1,
                fills: 1,
                stops: 2,
                shadows: 0,
                decorations: 0,
                glyphs: 0,
            }
        );
    }

    const fn rect() -> Rect {
        Rect::new(Point::new(0.0, 0.0), 10.0, 10.0)
    }

    const fn color() -> Rgba8 {
        Rgba8 {
            r: 1,
            g: 2,
            b: 3,
            a: 255,
        }
    }
}
