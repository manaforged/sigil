use crate::geometry::Rect;
use crate::renderer::split_encoder::SplitEncoder;
use crate::scene::command::{DrawPath, Transform};

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const SRGB_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const MAX_TARGETS: usize = 4;

struct Gpu {
    device: wgpu::Device,
    renderer: vello_gpu::Renderer,
    resources: vello_gpu::Resources,
}

struct Scratch {
    size: (u16, u16),
    unorm: wgpu::TextureView,
    srgb: wgpu::TextureView,
}

pub(crate) struct Rendered {
    pub(crate) view: wgpu::TextureView,
    pub(crate) size: (u32, u32),
    pub(crate) bounds: Rect,
}

#[derive(Clone, Copy)]
pub(crate) struct Run {
    pub(crate) size: (u16, u16),
    pub(crate) visible: Rect,
    pub(crate) opacity: f32,
}

#[derive(Default)]
pub(crate) struct VectorPass {
    gpu: Option<Gpu>,
    scene: Option<vello_gpu::Scene>,
    run: Option<Rect>,
    targets: Vec<Scratch>,
}

impl VectorPass {
    pub(crate) fn queue(&mut self, draw: &DrawPath, device: Transform, run: Run) {
        let scene = self
            .scene
            .get_or_insert_with(|| vello_gpu::Scene::new(run.size.0, run.size.1));
        if self.run.is_none() {
            scene.reset_and_resize(run.size.0, run.size.1);
        }
        crate::renderer::vector::record(scene, draw, device, run.opacity);
        self.run = Some(self.run.map_or(run.visible, |queued| queued.union(run.visible)));
    }

    pub(crate) fn contains(&self, view: &wgpu::TextureView) -> bool {
        self.targets
            .iter()
            .any(|target| target.unorm == *view || target.srgb == *view)
    }

    pub(crate) fn render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut SplitEncoder,
        linear: bool,
    ) -> Option<Rendered> {
        let bounds = self.run.take()?;
        let scene = self.scene.as_ref()?;
        let size = (scene.width(), scene.height());
        let target = scratch(&mut self.targets, device, size);
        let gpu = gpu(&mut self.gpu, device, size);
        let render_size = vello_gpu::RenderSize {
            width: size.0,
            height: size.1,
        };
        let unorm = target.unorm.clone();
        let view = if linear { target.srgb.clone() } else { unorm.clone() };
        let result = encoder.submit_with(|raw| {
            gpu.renderer.render(
                scene,
                &mut gpu.resources,
                device,
                queue,
                raw,
                &render_size,
                &unorm,
                None,
                &vello_gpu::TextureBindings::new(),
                vello_gpu::TargetInit::Clear(vello_gpu::ClearSettings::default()),
            )
        });
        result.ok()?;
        Some(Rendered {
            view,
            size: (u32::from(size.0), u32::from(size.1)),
            bounds,
        })
    }
}

fn gpu<'a>(slot: &'a mut Option<Gpu>, device: &wgpu::Device, size: (u16, u16)) -> &'a mut Gpu {
    if slot.as_ref().is_some_and(|gpu| gpu.device != *device) {
        *slot = None;
    }
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
        }
    })
}

fn scratch<'a>(targets: &'a mut Vec<Scratch>, device: &wgpu::Device, size: (u16, u16)) -> &'a Scratch {
    if let Some(found) = targets.iter().position(|target| target.size == size) {
        let target = targets.remove(found);
        targets.push(target);
    } else {
        if targets.len() >= MAX_TARGETS {
            let _ = targets.remove(0);
        }
        targets.push(new_scratch(device, size));
    }
    targets.last().expect("a scratch target was just pushed")
}

fn new_scratch(device: &wgpu::Device, size: (u16, u16)) -> Scratch {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("sigil-renderer vector target"),
        size: wgpu::Extent3d {
            width: u32::from(size.0),
            height: u32::from(size.1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[SRGB_FORMAT],
    });
    let unorm = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let srgb = texture.create_view(&wgpu::TextureViewDescriptor {
        format: Some(SRGB_FORMAT),
        ..wgpu::TextureViewDescriptor::default()
    });
    Scratch { size, unorm, srgb }
}
