use crate::hal::{Color, PhysicalSize, Renderer as _};
#[cfg(feature = "inspect")]
use crate::renderer::capture::{CaptureExtent, capture_texture, read_texture};
#[cfg(feature = "inspect")]
use crate::renderer::offscreen::OffscreenTarget;
use crate::renderer::{
    CommandScene, CpuRenderer, CpuSurface, FrameStats, RendererError, WgpuRenderer, WgpuSurface,
    into_window_handle,
};
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};

const ENV: &str = "SIGIL_BACKEND";

pub(super) struct Backend {
    inner: Inner,
    fallback: bool,
    #[cfg(target_arch = "wasm32")]
    device: Option<super::web::DeviceRequest>,
}

enum Inner {
    Wgpu(Box<WgpuRenderer>),
    Cpu(Box<CpuRenderer>),
}

pub(super) enum Surface {
    Wgpu(WgpuSurface),
    Cpu(CpuSurface),
}

impl Backend {
    pub(super) fn from_env() -> Result<Self, RendererError> {
        let choice = std::env::var(ENV).unwrap_or_default();
        match choice.as_str() {
            "cpu" => Ok(Self {
                inner: Inner::Cpu(Box::default()),
                fallback: false,
                #[cfg(target_arch = "wasm32")]
                device: None,
            }),
            "wgpu" | "gpu" => Ok(Self {
                inner: Inner::Wgpu(Box::new(WgpuRenderer::new()?)),
                fallback: false,
                #[cfg(target_arch = "wasm32")]
                device: None,
            }),
            "" | "auto" => Ok(Self {
                inner: Inner::Wgpu(Box::new(WgpuRenderer::new()?)),
                fallback: true,
                #[cfg(target_arch = "wasm32")]
                device: None,
            }),
            other => {
                tracing::warn!("sigil-app: {ENV}={other:?} is not cpu, wgpu, or auto; using auto");
                Ok(Self {
                    inner: Inner::Wgpu(Box::new(WgpuRenderer::new()?)),
                    fallback: true,
                    #[cfg(target_arch = "wasm32")]
                    device: None,
                })
            }
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub(super) fn request_device(&mut self, wake: impl Fn() + Send + Sync + 'static) {
        if let Inner::Wgpu(renderer) = &self.inner {
            let mut request = super::web::DeviceRequest::new(wake);
            request.start(renderer);
            self.device = Some(request);
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub(super) fn ready(&mut self) -> Result<bool, RendererError> {
        let Some(request) = &mut self.device else {
            return Ok(true);
        };
        if let Inner::Wgpu(renderer) = &self.inner
            && renderer.device_lost()
            && !request.waiting()
        {
            tracing::warn!("sigil-app: GPU device lost; requesting a new one");
            request.start(renderer);
        }
        let Some(result) = request.take() else {
            return Ok(!request.waiting());
        };
        match (result, &mut self.inner) {
            (Ok(parts), Inner::Wgpu(renderer)) => renderer.adopt_device(parts),
            (Ok(_), Inner::Cpu(_)) => {}
            (Err(err), _) if self.fallback => {
                tracing::warn!("sigil-app: {err}; using the CPU renderer");
                let mut cpu = CpuRenderer::new();
                cpu.set_transparent(self.is_transparent());
                self.inner = Inner::Cpu(Box::new(cpu));
                self.device = None;
            }
            (Err(err), _) => return Err(err),
        }
        Ok(true)
    }

    pub(super) const fn name(&self) -> &'static str {
        match self.inner {
            Inner::Wgpu(_) => "wgpu",
            Inner::Cpu(_) => "cpu",
        }
    }

    pub(super) const fn set_transparent(&mut self, transparent: bool) {
        match &mut self.inner {
            Inner::Wgpu(r) => r.set_transparent(transparent),
            Inner::Cpu(r) => r.set_transparent(transparent),
        }
    }

    pub(super) const fn is_transparent(&self) -> bool {
        match &self.inner {
            Inner::Wgpu(r) => r.is_transparent(),
            Inner::Cpu(r) => r.is_transparent(),
        }
    }

    pub(super) const fn last_frame_stats(&self) -> FrameStats {
        match &self.inner {
            Inner::Wgpu(r) => r.last_frame_stats(),
            Inner::Cpu(r) => r.last_frame_stats(),
        }
    }

    pub(super) fn create_surface<W>(
        &mut self,
        window: W,
        size: PhysicalSize,
    ) -> Result<Surface, RendererError>
    where
        W: HasWindowHandle + HasDisplayHandle + Clone + Send + Sync + 'static,
    {
        match &mut self.inner {
            Inner::Cpu(r) => Ok(Surface::Cpu(
                r.create_surface(into_window_handle(window), size)?,
            )),
            Inner::Wgpu(r) => match r.create_surface(into_window_handle(window.clone()), size) {
                Ok(surface) => Ok(Surface::Wgpu(surface)),
                Err(err) if self.fallback => {
                    tracing::warn!("sigil-app: {err}; using the CPU renderer");
                    let mut cpu = CpuRenderer::new();
                    cpu.set_transparent(r.is_transparent());
                    let surface = cpu.create_surface(into_window_handle(window), size)?;
                    self.inner = Inner::Cpu(Box::new(cpu));
                    Ok(Surface::Cpu(surface))
                }
                Err(err) => Err(err),
            },
        }
    }

    pub(super) fn resize(
        &mut self,
        surface: &mut Surface,
        size: PhysicalSize,
    ) -> Result<(), RendererError> {
        match (&mut self.inner, surface) {
            (Inner::Wgpu(r), Surface::Wgpu(s)) => r.resize(s, size),
            (Inner::Cpu(r), Surface::Cpu(s)) => r.resize(s, size),
            _ => Err(mismatch()),
        }
    }

    pub(super) fn render(
        &mut self,
        surface: &mut Surface,
        scene: &CommandScene,
        clear: Color,
        viewport: (f32, f32),
    ) -> Result<(), RendererError> {
        match (&mut self.inner, surface) {
            (Inner::Wgpu(r), Surface::Wgpu(s)) => r.render_command_scene(s, scene, clear, viewport),
            (Inner::Cpu(r), Surface::Cpu(s)) => r.render_command_scene(s, scene, clear, viewport),
            _ => Err(mismatch()),
        }
    }

    #[cfg(feature = "inspect")]
    pub(super) fn screenshot(
        &mut self,
        surface: &Surface,
        scene: &CommandScene,
        clear: Color,
        viewport: (f32, f32),
    ) -> Result<Vec<u8>, RendererError> {
        match (&mut self.inner, surface) {
            (Inner::Cpu(r), Surface::Cpu(s)) => r
                .render(scene, clear, viewport, s.size())?
                .encode_png()
                .map_err(|err| RendererError::Software(err.to_string())),
            (Inner::Wgpu(r), Surface::Wgpu(s)) => wgpu_shot(r, s, scene, clear, viewport),
            _ => Err(mismatch()),
        }
    }
}

#[cfg(feature = "inspect")]
pub(super) fn inspect_shot(
    app: &mut crate::app::embed::Host,
    renderer: &mut Backend,
    surface: &mut Option<Surface>,
    viewport: (f32, f32),
) -> Result<serde_json::Value, String> {
    let Some((cx, _, root)) = app.tree() else {
        return Err("no mounted tree".into());
    };
    let scene = crate::app::embed::emit_viewport_scene(cx, root, viewport);
    let Some(surface) = surface.as_mut() else {
        return Err("window surface is not initialized yet".into());
    };
    let clear = if renderer.is_transparent() {
        Color::rgba(0.0, 0.0, 0.0, 0.0)
    } else {
        Color::rgba(0.03, 0.04, 0.05, 1.0)
    };
    let size = surface_size(surface);
    let png = renderer
        .screenshot(surface, &scene, clear, viewport)
        .map_err(|err| err.to_string())?;
    Ok(serde_json::json!({
        "png": base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            png,
        ),
        "width": size.width,
        "height": size.height,
    }))
}

#[cfg(feature = "inspect")]
const fn surface_size(surface: &Surface) -> PhysicalSize {
    match surface {
        Surface::Wgpu(surface) => surface.size(),
        Surface::Cpu(surface) => surface.size(),
    }
}

#[cfg(feature = "inspect")]
#[allow(
    clippy::cast_precision_loss,
    reason = "mirrors the surface_text_scale formula used for on-screen frames"
)]
fn wgpu_shot(
    renderer: &mut WgpuRenderer,
    surface: &WgpuSurface,
    scene: &CommandScene,
    clear: Color,
    viewport: (f32, f32),
) -> Result<Vec<u8>, RendererError> {
    let (device, queue) = renderer
        .device()
        .zip(renderer.queue())
        .ok_or(RendererError::NotInitialized)
        .map(|(device, queue)| (device.clone(), queue.clone()))?;
    let size = surface.size();
    let extent = CaptureExtent {
        width: size.width,
        height: size.height,
    };
    let texture = capture_texture(&device, extent);
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let scale = size.width as f32 / viewport.0.max(1.0);
    renderer.render_command_scene_offscreen(
        OffscreenTarget {
            device: &device,
            queue: &queue,
            view: &view,
            format: crate::renderer::capture::FORMAT,
            viewport,
            scale,
        },
        scene,
        clear,
    )?;
    let rgba = read_texture(&device, &queue, &texture, extent)
        .map_err(|err| RendererError::Software(err.to_string()))?;
    let image = image::RgbaImage::from_raw(size.width, size.height, rgba)
        .ok_or(RendererError::InvalidViewport)?;
    let mut png = Vec::new();
    image
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .map_err(|err| RendererError::Software(err.to_string()))?;
    Ok(png)
}

fn mismatch() -> RendererError {
    RendererError::Software("surface was created by a different backend".to_owned())
}
