use std::collections::VecDeque;
use std::mem::take;
use std::sync::{Arc, OnceLock};

const MAX_IN_FLIGHT: usize = 3;
const MIN_QUERIES: u32 = 64;
const FRAME_START: &str = "frame start: clears and uploads";
const TIMESTAMP_BYTES: u64 = size_of::<u64>() as u64;

struct Readback {
    resolve: wgpu::Buffer,
    read: wgpu::Buffer,
    capacity: u32,
    period: f32,
    labels: Vec<&'static str>,
    mapped: Arc<OnceLock<bool>>,
}

impl Readback {
    fn new(device: &wgpu::Device, capacity: u32) -> Self {
        let size = (u64::from(capacity) * TIMESTAMP_BYTES)
            .next_multiple_of(wgpu::QUERY_RESOLVE_BUFFER_ALIGNMENT);
        let buffer = |label, usage| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size,
                usage,
                mapped_at_creation: false,
            })
        };
        Self {
            resolve: buffer(
                "sigil-renderer timestamp resolve",
                wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
            ),
            read: buffer(
                "sigil-renderer timestamp readback",
                wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            ),
            capacity,
            period: 1.0,
            labels: Vec::new(),
            mapped: Arc::new(OnceLock::new()),
        }
    }

    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "GPU tick deltas within one frame are far below f64 mantissa and u64 range"
    )]
    fn report(&self) {
        let bytes = (self.labels.len() as u64 + 1) * TIMESTAMP_BYTES;
        let Ok(view) = self.read.get_mapped_range(..bytes) else {
            return;
        };
        let ticks: Vec<u64> = view
            .as_chunks::<{ size_of::<u64>() }>()
            .0
            .iter()
            .map(|raw| u64::from_le_bytes(*raw))
            .collect();
        for (label, pair) in self.labels.iter().copied().zip(ticks.windows(2)) {
            let &[start, end] = pair else {
                continue;
            };
            if end < start {
                continue;
            }
            let ns = ((end - start) as f64 * f64::from(self.period)) as u64;
            crate::profile::gpu_pass(label, ns);
        }
    }
}

#[derive(Default)]
pub(crate) struct GpuTimer {
    queries: Option<wgpu::QuerySet>,
    capacity: u32,
    wanted: u32,
    period: f32,
    labels: Vec<&'static str>,
    resolved: Option<Readback>,
    free: Vec<Readback>,
    in_flight: VecDeque<Readback>,
    terminator: Option<wgpu::TextureView>,
}

impl GpuTimer {
    pub(crate) fn supported(device: &wgpu::Device) -> bool {
        device.features().contains(wgpu::Features::TIMESTAMP_QUERY)
    }

    pub(crate) fn begin(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) -> bool {
        self.drain(device);
        self.labels.clear();
        self.resolved = None;
        if self.in_flight.len() >= MAX_IN_FLIGHT {
            return false;
        }
        let target = self
            .wanted
            .saturating_add(1)
            .next_power_of_two()
            .clamp(MIN_QUERIES, wgpu::QUERY_SET_MAX_QUERIES);
        if self.queries.is_none() || target > self.capacity {
            self.queries = Some(device.create_query_set(&wgpu::QuerySetDescriptor {
                label: Some("sigil-renderer pass timestamps"),
                ty: wgpu::QueryType::Timestamp,
                count: target,
            }));
            self.capacity = target;
        }
        self.wanted = 0;
        self.period = queue.get_timestamp_period();
        true
    }

    pub(crate) fn next(&mut self, label: &'static str) -> Option<(wgpu::QuerySet, u32)> {
        self.wanted = self.wanted.saturating_add(1);
        let index = u32::try_from(self.labels.len()).ok()?;
        if index.checked_add(2)? > self.capacity {
            return None;
        }
        let queries = self.queries.clone()?;
        self.labels.push(label);
        Some((queries, index))
    }

    pub(crate) fn mark_start(&mut self, device: &wgpu::Device, encoder: &mut wgpu::CommandEncoder) {
        if let Some((queries, index)) = self.next(FRAME_START) {
            self.marker(device, encoder, &queries, index);
        }
    }

    pub(crate) fn terminate(&mut self, device: &wgpu::Device, encoder: &mut wgpu::CommandEncoder) {
        let Some(queries) = self.queries.clone() else {
            return;
        };
        let Ok(index) = u32::try_from(self.labels.len()) else {
            return;
        };
        if index == 0 || index >= self.capacity {
            return;
        }
        self.marker(device, encoder, &queries, index);
    }

    fn marker(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        queries: &wgpu::QuerySet,
        index: u32,
    ) {
        let view = self
            .terminator
            .get_or_insert_with(|| {
                device
                    .create_texture(&wgpu::TextureDescriptor {
                        label: Some("sigil-renderer timestamp terminator"),
                        size: wgpu::Extent3d {
                            width: 1,
                            height: 1,
                            depth_or_array_layers: 1,
                        },
                        mip_level_count: 1,
                        sample_count: 1,
                        dimension: wgpu::TextureDimension::D2,
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                        view_formats: &[],
                    })
                    .create_view(&wgpu::TextureViewDescriptor::default())
            })
            .clone();
        drop(encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("sigil-renderer timestamp terminator"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Discard,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: Some(wgpu::RenderPassTimestampWrites {
                query_set: queries,
                beginning_of_pass_write_index: Some(index),
                end_of_pass_write_index: None,
            }),
            occlusion_query_set: None,
            multiview_mask: None,
        }));
    }

    pub(crate) fn resolve(&mut self, device: &wgpu::Device, encoder: &mut wgpu::CommandEncoder) {
        let Some(queries) = self.queries.as_ref() else {
            return;
        };
        let Some(used) = u32::try_from(self.labels.len())
            .ok()
            .filter(|passes| *passes > 0)
            .and_then(|passes| passes.checked_add(1))
        else {
            return;
        };
        let mut readback = self
            .free
            .iter()
            .position(|readback| readback.capacity >= used)
            .map_or_else(
                || Readback::new(device, self.capacity),
                |index| self.free.swap_remove(index),
            );
        encoder.resolve_query_set(queries, 0..used, &readback.resolve, 0);
        encoder.copy_buffer_to_buffer(
            &readback.resolve,
            0,
            &readback.read,
            0,
            u64::from(used) * TIMESTAMP_BYTES,
        );
        readback.labels = take(&mut self.labels);
        readback.period = self.period;
        readback.mapped = Arc::new(OnceLock::new());
        self.resolved = Some(readback);
    }

    pub(crate) fn submitted(&mut self) {
        let Some(readback) = self.resolved.take() else {
            return;
        };
        let mapped = Arc::clone(&readback.mapped);
        readback
            .read
            .map_async(wgpu::MapMode::Read, .., move |result| {
                let _ = mapped.set(result.is_ok());
            });
        self.in_flight.push_back(readback);
    }

    pub(crate) fn drain(&mut self, device: &wgpu::Device) {
        if self.in_flight.is_empty() {
            return;
        }
        let _ = device.poll(wgpu::PollType::Poll);
        while let Some(ok) = self
            .in_flight
            .front()
            .and_then(|readback| readback.mapped.get().copied())
        {
            let Some(readback) = self.in_flight.pop_front() else {
                break;
            };
            if ok {
                readback.report();
                readback.read.unmap();
            }
            if ok && self.free.len() < MAX_IN_FLIGHT {
                self.free.push(readback);
            }
        }
    }
}
