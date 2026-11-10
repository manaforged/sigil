use sigil::Host;
use sigil::profile::{self, Backend, BackendStatus, FallbackCause, Phase};
use sigil::renderer::{ScreenSubmission, WgpuRenderer};

use crate::gpu::{
    CLEAR, FORMAT, Gpu, create_capture_texture, headless_renderer, read_pixels, shared_device,
};
use crate::{Error, ErrorKind};

pub(super) const GPU_STATUS: BackendStatus = BackendStatus {
    kind: Backend::Gpu,
    fallback: FallbackCause::None,
};

pub(super) struct GpuSurface {
    pub(super) viewport: (f32, f32),
    scale: f32,
    gpu: &'static Gpu,
    renderer: WgpuRenderer,
    size_px: (u32, u32),
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    error: Option<Error>,
}

impl GpuSurface {
    pub(super) fn open(width: u32, height: u32, scale: f32) -> Result<Option<Self>, Error> {
        let opened = headless_renderer().and_then(|renderer| Ok((renderer, shared_device()?)));
        let (renderer, gpu) = match opened {
            Ok((Some(renderer), Some(gpu))) => (renderer, gpu),
            Ok(_) => return Ok(None),
            Err(err) if err.is_no_adapter() => return Ok(None),
            Err(err) => return Err(err),
        };
        let size_px = |extent: u32| (crate::extent_f32(extent) * scale).round();
        let size = (physical(size_px(width)), physical(size_px(height)));
        let texture = create_capture_texture(&gpu.0, size.0, size.1);
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        Ok(Some(Self {
            viewport: (crate::extent_f32(width), crate::extent_f32(height)),
            scale,
            gpu,
            renderer,
            size_px: size,
            texture,
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
            self.error = Some(Error::caused(ErrorKind::Render, err));
            return;
        }
        let waited = {
            let _wait = profile::span(Phase::GpuWait);
            device.poll(wgpu::PollType::wait_indefinitely())
        };
        if let Err(err) = waited {
            self.error = Some(Error::caused(ErrorKind::Render, err));
            return;
        }
        self.renderer.drain_gpu_timings();
    }

    pub(super) fn image(&self) -> Result<image::RgbaImage, Error> {
        let (width, height) = self.size_px;
        let rgba = read_pixels(&self.gpu.0, &self.gpu.1, &self.texture, width, height)?;
        image::RgbaImage::from_raw(width, height, rgba).ok_or_else(|| {
            Error::text(
                ErrorKind::Readback,
                format_args!("readback is not {width}x{height} rgba"),
            )
        })
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
