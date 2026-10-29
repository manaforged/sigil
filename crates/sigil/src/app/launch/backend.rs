use crate::geometry::PhysicalSize;
use crate::hal::Renderer as _;
use crate::profile::{BackendKind, BackendStatus, FallbackCause};
#[cfg(any(feature = "inspect", not(target_arch = "wasm32")))]
use crate::renderer::capture::{ImageRenderer, ImageTarget, render_to_image};
use crate::renderer::frame_stats::FrameStats;
use crate::renderer::{
    CpuRenderer, CpuSurface, RendererError, WgpuRenderer, WgpuSurface, into_window_handle,
};
use crate::scene::CommandScene;
use crate::style::Color;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};

const ENV: &str = "SIGIL_BACKEND";
const SURFACE_CLEAR: Color = Color::rgba(0.03, 0.04, 0.05, 1.0);
const TRANSPARENT_CLEAR: Color = Color::rgba(0.0, 0.0, 0.0, 0.0);

pub(in crate::app) struct Backend {
    inner: Inner,
    fallback: bool,
    status: BackendStatus,
    #[cfg(target_arch = "wasm32")]
    device: Option<DeviceRequest>,
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
    pub(in crate::app) fn from_env() -> Result<Self, RendererError> {
        let value = std::env::var(ENV).unwrap_or_default();
        let (cause, fallback) = match value.as_str() {
            "cpu" => (FallbackCause::ForcedByEnv, false),
            "wgpu" | "gpu" => (FallbackCause::None, false),
            "" | "auto" => (FallbackCause::None, true),
            _ => {
                tracing::warn!("sigil-app: {ENV}={value:?} is not cpu, wgpu, or auto; using auto");
                (FallbackCause::UnknownEnv, true)
            }
        };
        let (inner, kind) = if cause == FallbackCause::ForcedByEnv {
            (Inner::Cpu(Box::default()), BackendKind::Cpu)
        } else {
            (
                Inner::Wgpu(Box::new(WgpuRenderer::new()?)),
                BackendKind::Gpu,
            )
        };
        Ok(Self {
            inner,
            fallback,
            status: BackendStatus {
                kind,
                fallback: cause,
            },
            #[cfg(target_arch = "wasm32")]
            device: None,
        })
    }

    pub(super) const fn status(&self) -> BackendStatus {
        self.status
    }

    pub(in crate::app) const fn clear_color(&self) -> Color {
        if self.is_transparent() {
            TRANSPARENT_CLEAR
        } else {
            SURFACE_CLEAR
        }
    }

    fn settle(&mut self, status: BackendStatus) {
        let fallback = match (status.fallback, self.status.fallback) {
            (FallbackCause::None, earlier) => earlier,
            (cause, _) => cause,
        };
        self.status = BackendStatus {
            kind: status.kind,
            fallback,
        };
        if fallback != FallbackCause::None {
            tracing::warn!("sigil-app: renderer degraded: {}", self.status);
        }
    }

    fn adopt_gpu_status(&mut self) {
        if let Inner::Wgpu(renderer) = &self.inner
            && let Some(status) = renderer.backend_status()
        {
            self.settle(status);
        }
    }

    fn fall_back_to_cpu(&mut self, err: &RendererError) -> CpuRenderer {
        tracing::warn!("sigil-app: {err}; using the CPU renderer");
        let fallback = match err {
            RendererError::NoAdapter => FallbackCause::NoAdapter,
            _ => FallbackCause::DeviceError,
        };
        self.settle(BackendStatus {
            kind: BackendKind::Cpu,
            fallback,
        });
        let mut cpu = CpuRenderer::new();
        cpu.set_transparent(self.is_transparent());
        cpu
    }

    #[cfg(target_arch = "wasm32")]
    pub(super) fn request_device(&mut self, wake: impl Fn() + Send + Sync + 'static) {
        if let Inner::Wgpu(renderer) = &self.inner {
            let mut request = DeviceRequest::new(wake);
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
            (Ok(parts), Inner::Wgpu(renderer)) => {
                renderer.adopt_device(parts);
                self.adopt_gpu_status();
            }
            (Ok(_), Inner::Cpu(_)) => {}
            (Err(err), _) if self.fallback => {
                let cpu = self.fall_back_to_cpu(&err);
                self.inner = Inner::Cpu(Box::new(cpu));
                self.device = None;
            }
            (Err(err), _) => return Err(err),
        }
        Ok(true)
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
            Inner::Wgpu(r) => r.last_frame_stats,
            Inner::Cpu(r) => r.stats,
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
        let attempt = match &mut self.inner {
            Inner::Cpu(r) => {
                return Ok(Surface::Cpu(
                    r.create_surface(into_window_handle(window), size)?,
                ));
            }
            Inner::Wgpu(r) => r.create_surface(into_window_handle(window.clone()), size),
        };
        match attempt {
            Ok(surface) => {
                self.adopt_gpu_status();
                Ok(Surface::Wgpu(surface))
            }
            Err(err) if self.fallback => {
                let mut cpu = self.fall_back_to_cpu(&err);
                let surface = cpu.create_surface(into_window_handle(window), size)?;
                self.inner = Inner::Cpu(Box::new(cpu));
                Ok(Surface::Cpu(surface))
            }
            Err(err) => Err(err),
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
}

fn mismatch() -> RendererError {
    RendererError::Software("surface was created by a different backend".to_owned())
}

#[cfg(any(feature = "inspect", not(target_arch = "wasm32")))]
impl Backend {
    pub(in crate::app) fn render_image(
        &mut self,
        scene: &CommandScene,
        target: ImageTarget,
    ) -> Result<image::RgbaImage, RendererError> {
        let (attempt, had_device) = match &mut self.inner {
            Inner::Cpu(r) => return render_to_image(ImageRenderer::Cpu(r), scene, target),
            Inner::Wgpu(r) => {
                let had_device = r.device().is_some();
                (
                    render_to_image(ImageRenderer::Gpu(r), scene, target),
                    had_device,
                )
            }
        };
        match attempt {
            Ok(image) => {
                self.adopt_gpu_status();
                Ok(image)
            }
            Err(err) if self.fallback && !had_device => {
                let mut cpu = self.fall_back_to_cpu(&err);
                let image = render_to_image(ImageRenderer::Cpu(&mut cpu), scene, target);
                self.inner = Inner::Cpu(Box::new(cpu));
                image
            }
            Err(err) => Err(err),
        }
    }
}

#[cfg(feature = "inspect")]
pub(super) fn inspect_shot(
    app: &mut crate::app::embed::Host,
    renderer: &mut Backend,
    surface: Option<&Surface>,
    viewport: (f32, f32),
) -> Result<serde_json::Value, String> {
    let Some((cx, _, root)) = app.tree() else {
        return Err("no mounted tree".into());
    };
    let scene = super::scene_cache::window_scene(cx, root, viewport, !renderer.is_transparent());
    let size = match surface {
        Some(Surface::Wgpu(surface)) => surface.size(),
        Some(Surface::Cpu(surface)) => surface.size(),
        None => return Err("window surface is not initialized yet".into()),
    };
    let clear = renderer.clear_color();
    let image = renderer
        .render_image(
            &scene,
            ImageTarget {
                size,
                viewport,
                clear,
            },
        )
        .map_err(|err| err.to_string())?;
    let mut png = Vec::new();
    image
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .map_err(|err| format!("software renderer failed: {err}"))?;
    Ok(serde_json::json!({
        "png": base64::Engine::encode(&base64::engine::general_purpose::STANDARD, png),
        "width": size.width,
        "height": size.height,
    }))
}

#[cfg(target_arch = "wasm32")]
type Slot =
    std::rc::Rc<std::cell::RefCell<Option<Result<crate::renderer::DeviceParts, RendererError>>>>;

#[cfg(target_arch = "wasm32")]
struct DeviceRequest {
    slot: Slot,
    wake: crate::renderer::OnLost,
    waiting: bool,
}

#[cfg(target_arch = "wasm32")]
impl DeviceRequest {
    fn new(wake: impl Fn() + Send + Sync + 'static) -> Self {
        Self {
            slot: Slot::default(),
            wake: std::sync::Arc::new(wake),
            waiting: false,
        }
    }

    fn start(&mut self, renderer: &WgpuRenderer) {
        let out = std::rc::Rc::clone(&self.slot);
        let wake = std::sync::Arc::clone(&self.wake);
        let request = renderer.request_device(std::sync::Arc::clone(&self.wake));
        wasm_bindgen_futures::spawn_local(async move {
            *out.borrow_mut() = Some(request.await);
            wake();
        });
        self.waiting = true;
    }

    const fn waiting(&self) -> bool {
        self.waiting
    }

    fn take(&mut self) -> Option<Result<crate::renderer::DeviceParts, RendererError>> {
        let result = self.slot.borrow_mut().take()?;
        self.waiting = false;
        Some(result)
    }
}
