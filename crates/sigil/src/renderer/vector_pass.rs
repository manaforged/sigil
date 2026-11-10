#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "device pixel sides fit in u16 and f32"
)]

use crate::geometry::Rect;
use crate::renderer::cache_limits::CacheLimits;
use crate::renderer::split_encoder::SplitEncoder;
use crate::scene::command::{DrawPath, Transform};

mod linear;

use linear::Linearize;

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const SRGB_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

struct Gpu {
    device: wgpu::Device,
    renderer: vello_gpu::Renderer,
    resources: vello_gpu::Resources,
    linearize: Linearize,
}

#[derive(Clone, PartialEq)]
struct Queued {
    draw: DrawPath,
    device: Transform,
    opacity: f32,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct Pixels {
    origin: (u16, u16),
    size: (u16, u16),
}

struct Retained {
    pixels: Pixels,
    used: u64,
    draws: Vec<Queued>,
    unorm: wgpu::TextureView,
    linear: wgpu::TextureView,
}

pub(crate) struct Rendered {
    pub(crate) view: wgpu::TextureView,
    pub(crate) origin: (u32, u32),
    pub(crate) size: (u32, u32),
    pub(crate) bounds: Rect,
}

#[derive(Clone, Copy)]
pub(crate) struct Run {
    pub(crate) size: (u16, u16),
    pub(crate) scale: f32,
    pub(crate) visible: Rect,
    pub(crate) opacity: f32,
}

#[derive(Default)]
pub(crate) struct VectorPass {
    gpu: Option<Gpu>,
    scene: Option<vello_gpu::Scene>,
    pending: Vec<Queued>,
    run: Option<(Rect, Run)>,
    retained: Vec<Retained>,
    frame: u64,
}

impl VectorPass {
    pub(crate) fn begin_frame(&mut self) {
        self.frame += 1;
        let stale = CacheLimits::DEFAULT.vector_stale_frames;
        let frame = self.frame;
        self.retained.retain(|kept| kept.used + stale >= frame);
    }

    pub(crate) fn queue(&mut self, draw: &DrawPath, device: Transform, run: Run) {
        self.pending.push(Queued {
            draw: draw.clone(),
            device,
            opacity: run.opacity,
        });
        let bounds = self
            .run
            .map_or(run.visible, |(queued, _)| queued.union(run.visible));
        self.run = Some((bounds, run));
    }

    pub(crate) fn contains(&self, view: &wgpu::TextureView) -> bool {
        self.retained
            .iter()
            .any(|target| target.unorm == *view || target.linear == *view)
    }

    pub(crate) fn render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut SplitEncoder,
        linear: bool,
    ) -> Option<Rendered> {
        let (bounds, run) = self.run.take()?;
        let draws = std::mem::take(&mut self.pending);
        let pixels = pixels(bounds, run)?;
        if self.gpu.as_ref().is_some_and(|gpu| gpu.device != *device) {
            self.gpu = None;
            self.retained.clear();
        }
        let found = self.find(pixels, draws, (device, queue), encoder)?;
        let target = self.retained.get(found)?;
        Some(Rendered {
            view: if linear {
                target.linear.clone()
            } else {
                target.unorm.clone()
            },
            origin: (u32::from(pixels.origin.0), u32::from(pixels.origin.1)),
            size: (u32::from(pixels.size.0), u32::from(pixels.size.1)),
            bounds,
        })
    }

    fn find(
        &mut self,
        pixels: Pixels,
        draws: Vec<Queued>,
        (device, queue): (&wgpu::Device, &wgpu::Queue),
        encoder: &mut SplitEncoder,
    ) -> Option<usize> {
        if let Some(hit) = self
            .retained
            .iter()
            .position(|kept| kept.pixels == pixels && kept.draws == draws)
        {
            self.pending = draws;
            self.pending.clear();
            if let Some(kept) = self.retained.get_mut(hit) {
                kept.used = self.frame;
            }
            return Some(hit);
        }
        let mut target = self.slot(device, pixels);
        target.pixels = pixels;
        target.used = self.frame;
        let painted = self.draw(&draws, &target, (device, queue), encoder);
        target.draws = if painted { draws } else { Vec::new() };
        self.retained.push(target);
        painted.then(|| self.retained.len() - 1)
    }

    fn slot(&mut self, device: &wgpu::Device, pixels: Pixels) -> Retained {
        let budget = CacheLimits::DEFAULT.vector_pixels;
        let area = |size: (u16, u16)| u64::from(size.0) * u64::from(size.1);
        let mut held: u64 = self
            .retained
            .iter()
            .map(|kept| area(kept.pixels.size))
            .sum();
        let mut reused = None;
        while held + area(pixels.size) > budget {
            let Some(oldest) = self.oldest() else {
                break;
            };
            let kept = self.retained.swap_remove(oldest);
            held -= area(kept.pixels.size);
            if kept.pixels.size == pixels.size {
                reused = Some(kept);
            }
        }
        reused.unwrap_or_else(|| new_target(device, pixels))
    }

    fn oldest(&self) -> Option<usize> {
        self.retained
            .iter()
            .enumerate()
            .filter(|(_, kept)| kept.used < self.frame)
            .min_by_key(|(_, kept)| kept.used)
            .map(|(index, _)| index)
    }

    fn draw(
        &mut self,
        draws: &[Queued],
        target: &Retained,
        (device, queue): (&wgpu::Device, &wgpu::Queue),
        encoder: &mut SplitEncoder,
    ) -> bool {
        let (width, height) = target.pixels.size;
        let scene = self
            .scene
            .get_or_insert_with(|| vello_gpu::Scene::new(width, height));
        scene.reset_and_resize(width, height);
        let (x, y) = target.pixels.origin;
        for queued in draws {
            let mut placed = queued.device;
            placed[0][2] -= f32::from(x);
            placed[1][2] -= f32::from(y);
            crate::renderer::vector::record(scene, &queued.draw, placed, queued.opacity);
        }
        let gpu = gpu(&mut self.gpu, device, target.pixels.size);
        let render_size = vello_gpu::RenderSize { width, height };
        let result = encoder.submit_with(|raw| {
            gpu.renderer.render(
                scene,
                &mut gpu.resources,
                device,
                queue,
                raw,
                &render_size,
                &target.unorm,
                None,
                &vello_gpu::TextureBindings::new(),
                vello_gpu::TargetInit::Clear(vello_gpu::ClearSettings::default()),
            )?;
            gpu.linearize
                .run(device, raw, &target.unorm, &target.linear);
            Ok::<(), vello_gpu::RenderError>(())
        });
        result.is_ok()
    }
}

fn pixels(bounds: Rect, run: Run) -> Option<Pixels> {
    let edge = |value: f32, limit: u16| (value * run.scale).clamp(0.0, f32::from(limit));
    let left = edge(bounds.origin.x, run.size.0).floor();
    let top = edge(bounds.origin.y, run.size.1).floor();
    let right = edge(bounds.origin.x + bounds.width, run.size.0).ceil();
    let bottom = edge(bounds.origin.y + bounds.height, run.size.1).ceil();
    let size = ((right - left) as u16, (bottom - top) as u16);
    (size.0 > 0 && size.1 > 0).then_some(Pixels {
        origin: (left as u16, top as u16),
        size,
    })
}

fn gpu<'a>(slot: &'a mut Option<Gpu>, device: &wgpu::Device, size: (u16, u16)) -> &'a mut Gpu {
    slot.get_or_insert_with(|| {
        let config = vello_gpu::RenderTargetConfig {
            format: FORMAT,
            width: size.0,
            height: size.1,
        };
        let (renderer, resources) = vello_gpu::Renderer::new(device, &config);
        Gpu {
            device: device.clone(),
            renderer,
            resources,
            linearize: Linearize::new(device, SRGB_FORMAT),
        }
    })
}

fn new_target(device: &wgpu::Device, pixels: Pixels) -> Retained {
    let view = |label, format| {
        device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d {
                    width: u32::from(pixels.size.0),
                    height: u32::from(pixels.size.1),
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            })
            .create_view(&wgpu::TextureViewDescriptor::default())
    };
    Retained {
        pixels,
        used: 0,
        draws: Vec::new(),
        unorm: view("sigil-renderer vector target", FORMAT),
        linear: view("sigil-renderer linear vector target", SRGB_FORMAT),
    }
}
