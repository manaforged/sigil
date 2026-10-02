use sigil::Host;
use sigil::profile::{self, BackendKind, BackendStatus, FallbackCause, Phase};
use sigil::renderer::{ScreenSubmission, WgpuRenderer};
use sigil::style::Color;

use crate::Error;
use crate::gpu::{FORMAT, Gpu, create_capture_texture, headless_renderer, shared_device};

const CLEAR: Color = Color::rgba(0.0, 0.0, 0.0, 1.0);

pub(super) const GPU_STATUS: BackendStatus = BackendStatus {
    kind: BackendKind::Gpu,
    fallback: FallbackCause::None,
};

pub(super) struct GpuSurface {
    pub(super) viewport: (f32, f32),
    scale: f32,
    gpu: &'static Gpu,
    renderer: WgpuRenderer,
    _texture: wgpu::Texture,
    view: wgpu::TextureView,
    error: Option<Error>,
}

impl GpuSurface {
    pub(super) fn open(width: u32, height: u32, scale: f32) -> Result<Option<Self>, Error> {
        let opened = headless_renderer().and_then(|renderer| Ok((renderer, shared_device()?)));
        let (renderer, gpu) = match opened {
            Ok((Some(renderer), Some(gpu))) => (renderer, gpu),
            Ok(_) | Err(Error::NoAdapter(_)) => return Ok(None),
            Err(err) => return Err(err),
        };
        let size_px = |extent: u32| (crate::extent_f32(extent) * scale).round();
        let texture =
            create_capture_texture(&gpu.0, physical(size_px(width)), physical(size_px(height)));
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        Ok(Some(Self {
            viewport: (crate::extent_f32(width), crate::extent_f32(height)),
            scale,
            gpu,
            renderer,
            _texture: texture,
            view,
            error: None,
        }))
    }

    pub(super) fn paint(&mut self, host: &mut Host) {
        let _paint = profile::span(Phase::Paint);
        let scene = {
            let _emit = profile::span(Phase::Emit);
            host.emit(self.viewport)
        };
        let (device, queue) = (&self.gpu.0, &self.gpu.1);
        let target = ScreenSubmission {
            device,
            queue,
            view: &self.view,
            format: FORMAT,
            viewport: self.viewport,
            scale: self.scale,
            clear: Some(CLEAR),
        };
        let submitted = {
            let _render = profile::span(Phase::RenderCpu);
            self.renderer.submit_screen(&target, &scene)
        };
        if let Err(err) = submitted {
            self.error = Some(Error::Render(err.to_string()));
            return;
        }
        let waited = {
            let _wait = profile::span(Phase::GpuWait);
            device.poll(wgpu::PollType::wait_indefinitely())
        };
        if let Err(err) = waited {
            self.error = Some(Error::Render(err.to_string()));
            return;
        }
        self.renderer.drain_gpu_timings();
    }

    pub(super) fn take_error(&mut self) -> Result<(), Error> {
        self.error.take().map_or(Ok(()), Err)
    }
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "a rounded, clamped texture extent fits u32"
)]
fn physical(extent: f32) -> u32 {
    extent.clamp(1.0, f32::from(u16::MAX)) as u32
}
