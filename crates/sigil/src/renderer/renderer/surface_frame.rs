use std::borrow::Cow;

use crate::geometry::{Point, Rect};
use crate::renderer::damage::{self, FrameKey, FullReason, Plan, Retained};
use crate::renderer::device_state::ReadyState;
use crate::renderer::error::RendererError;
use crate::renderer::frame_stats::FrameStats;
use crate::renderer::layer_pool::LayerHandle;
use crate::renderer::offscreen::OffscreenTarget;
use crate::renderer::scene_command::CommandScene;
use crate::renderer::split_encoder::SplitEncoder;
use crate::renderer::surface::WgpuSurface;
use crate::style::Color;

use super::submit::{acquire_frame, physical_pixels, surface_text_scale};

pub(crate) struct SurfaceFrame<'a> {
    pub(crate) scene: &'a CommandScene,
    pub(crate) clear: Color,
    pub(crate) viewport: (f32, f32),
}

struct Recording<'a> {
    encoder: SplitEncoder,
    layers: Vec<LayerHandle>,
    stats: &'a mut FrameStats,
    retained: &'a mut Option<Retained>,
    scale: f32,
    format: wgpu::TextureFormat,
}

impl ReadyState {
    pub(crate) fn render_surface(
        &mut self,
        surface: &mut WgpuSurface,
        request: &SurfaceFrame<'_>,
        stats: &mut FrameStats,
        retained: &mut Option<Retained>,
    ) -> Result<(), RendererError> {
        let Some((frame, view)) = acquire_frame(self, surface)? else {
            return Ok(());
        };
        let encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("sigil-renderer surface frame encoder"),
            });
        let mut recording = Recording {
            encoder: SplitEncoder::new(&self.device, &self.queue, encoder),
            layers: Vec::new(),
            stats,
            retained,
            scale: surface_text_scale(surface, request.viewport),
            format: surface.config.format,
        };
        recording.encoder.reuse_uploads(&mut self.uploads);
        recording.encoder.time_passes(&mut self.gpu_timer);
        let back = (request.clear.a >= 1.0)
            .then(|| backbuffer(&self.device, surface))
            .flatten();
        let encode = crate::profile::span(crate::profile::Phase::RenderEncode);
        let result = if let Some(back) = back {
            self.draw_retained(&mut recording, &back, &frame.texture, request)
        } else {
            *recording.retained = None;
            let reason = if request.clear.a < 1.0 {
                FullReason::TranslucentClear
            } else {
                FullReason::NoBackbuffer
            };
            self.draw(&mut recording, &view, request, &Plan::Full(reason))
        };
        let Recording {
            encoder: split,
            layers,
            ..
        } = recording;
        drop(encode);
        let _submit = crate::profile::span(crate::profile::Phase::RenderSubmit);
        self.finish(split, frame, layers);
        result
    }

    fn finish(
        &mut self,
        split: SplitEncoder,
        frame: wgpu::SurfaceTexture,
        layers: Vec<LayerHandle>,
    ) {
        let (uploads, timer) = split.submit();
        self.uploads = Some(uploads);
        if timer.is_some() {
            self.gpu_timer = timer;
        }
        self.queue.present(frame);
        if let Some(pool) = self.pipelines.layer_pool.as_mut() {
            for handle in layers {
                pool.release(handle);
            }
        }
    }

    fn draw_retained(
        &mut self,
        recording: &mut Recording<'_>,
        back: &wgpu::Texture,
        screen: &wgpu::Texture,
        request: &SurfaceFrame<'_>,
    ) -> Result<(), RendererError> {
        let view = back.create_view(&wgpu::TextureViewDescriptor::default());
        let key = FrameKey {
            target: back.clone(),
            viewport: request.viewport,
            scale: recording.scale,
            clear: request.clear,
        };
        let plan = recording
            .retained
            .as_ref()
            .map_or(Plan::Full(FullReason::FirstFrame), |previous| {
                previous.plan(&key, request.scene)
            });
        let drawn = self.draw(recording, &view, request, &plan);
        recording.encoder.copy_texture(back, screen);
        *recording.retained = Some(Retained::new(key, request.scene.clone()));
        drawn
    }

    fn draw(
        &mut self,
        recording: &mut Recording<'_>,
        view: &wgpu::TextureView,
        request: &SurfaceFrame<'_>,
        plan: &Plan,
    ) -> Result<(), RendererError> {
        let scale = recording.scale;
        damage::note_plan(plan);
        let (scene, redrawn) = match plan {
            Plan::Unchanged => return Ok(()),
            Plan::Regions(regions) => (
                Cow::Owned(damage::clipped(request.scene, regions, request.clear)),
                regions
                    .iter()
                    .map(|region| physical_pixels(*region, scale))
                    .sum(),
            ),
            Plan::Full(_) => {
                recording.encoder.clear(view, request.clear);
                let full = Rect::new(Point::new(0.0, 0.0), request.viewport.0, request.viewport.1);
                (Cow::Borrowed(request.scene), physical_pixels(full, scale))
            }
        };
        self.begin_frame();
        let target = OffscreenTarget {
            device: &self.device,
            queue: &self.queue,
            view,
            format: recording.format,
            viewport: request.viewport,
            scale,
        };
        let result = self.pipelines.walk(
            &mut recording.encoder,
            &mut recording.layers,
            target,
            &scene,
            recording.stats,
        );
        recording.stats.redrawn_pixels = redrawn;
        result
    }
}

fn backbuffer(device: &wgpu::Device, surface: &mut WgpuSurface) -> Option<wgpu::Texture> {
    let config = &surface.config;
    if !config.usage.contains(wgpu::TextureUsages::COPY_DST) {
        return None;
    }
    let fits = surface.backbuffer.as_deref().is_some_and(|texture| {
        texture.width() == config.width
            && texture.height() == config.height
            && texture.format() == config.format
    });
    if !fits {
        surface.backbuffer = Some(Box::new(device.create_texture(&wgpu::TextureDescriptor {
            label: Some("sigil-renderer surface back buffer"),
            size: wgpu::Extent3d {
                width: config.width,
                height: config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: config.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        })));
    }
    surface.backbuffer.as_deref().cloned()
}
