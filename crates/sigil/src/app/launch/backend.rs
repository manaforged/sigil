use crate::geometry::PhysicalSize;
use crate::hal::Renderer as _;
use crate::profile::{BackendKind, BackendStatus, FallbackCause};
#[cfg(feature = "inspect")]
use crate::renderer::capture::{ImageRenderer, ImageTarget, render_to_image};
use crate::renderer::frame_stats::FrameStats;
use crate::renderer::{CpuRenderer, CpuSurface, WgpuRenderer, WgpuSurface, into_window_handle};
use crate::scene::CommandScene;
use crate::style::Color;
use crate::{Error, ErrorKind};
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};

#[cfg(feature = "inspect")]
mod shot;

#[cfg(feature = "inspect")]
pub(super) use shot::inspect_shot;

const ENV: &str = "SIGIL_BACKEND";
use crate::renderer::SURFACE_CLEAR;
const TRANSPARENT_CLEAR: Color = Color::rgba(0.0, 0.0, 0.0, 0.0);

pub(in crate::app) struct Backend {
    inner: BackendInner,
    fallback: bool,
    status: BackendStatus,
    #[cfg(target_arch = "wasm32")]
    device: Option<DeviceRequest>,
}

enum BackendInner {
    Wgpu(Box<WgpuRenderer>),
    Cpu(Box<CpuRenderer>),
}

pub(super) enum RenderSurface {
    Wgpu(WgpuSurface),
    Cpu(CpuSurface),
}

impl Backend {
    pub(in crate::app) fn from_env() -> Self {
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
            (BackendInner::Cpu(Box::default()), BackendKind::Cpu)
        } else {
            (BackendInner::Wgpu(Box::default()), BackendKind::Gpu)
        };
        Self {
            inner,
            fallback,
            status: BackendStatus {
                kind,
                fallback: cause,
            },
            #[cfg(target_arch = "wasm32")]
            device: None,
        }
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
        if let BackendInner::Wgpu(renderer) = &self.inner
            && let Some(status) = renderer.backend_status()
        {
            self.settle(status);
        }
    }

    fn fall_back_to_cpu(&mut self, err: &Error) -> CpuRenderer {
        tracing::warn!("sigil-app: {err}; using the CPU renderer");
        let fallback = if err.kind() == ErrorKind::NoAdapter {
            FallbackCause::NoAdapter
        } else {
            FallbackCause::DeviceError
        };
        self.settle(BackendStatus {
            kind: BackendKind::Cpu,
            fallback,
        });
        let mut cpu = CpuRenderer::new();
        cpu.set_transparent(self.is_transparent());
        cpu
    }

    fn use_cpu(&mut self, cpu: CpuRenderer) {
        self.inner = BackendInner::Cpu(Box::new(cpu));
        #[cfg(target_arch = "wasm32")]
        {
            self.device = None;
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn set_wake(&mut self, wake: impl Fn() + Send + Sync + 'static) {
        if let BackendInner::Wgpu(renderer) = &mut self.inner {
            renderer.set_on_lost(std::sync::Arc::new(wake));
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn recover(&mut self, surface: &mut Option<RenderSurface>) -> Result<bool, Error> {
        let mismatched = matches!(
            (&self.inner, &*surface),
            (BackendInner::Wgpu(_), Some(RenderSurface::Cpu(_)))
                | (BackendInner::Cpu(_), Some(RenderSurface::Wgpu(_)))
        );
        if mismatched {
            *surface = None;
        }
        let (BackendInner::Wgpu(renderer), Some(RenderSurface::Wgpu(gpu))) =
            (&mut self.inner, surface.as_mut())
        else {
            return Ok(false);
        };
        match renderer.recover_lost_device(gpu) {
            Ok(rebuilt) => {
                if rebuilt {
                    self.adopt_gpu_status();
                }
                Ok(rebuilt)
            }
            Err(err) if self.fallback => {
                let cpu = self.fall_back_to_cpu(&err);
                self.use_cpu(cpu);
                *surface = None;
                Ok(true)
            }
            Err(err) => Err(err),
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub(super) fn request_device(&mut self, wake: impl Fn() + Send + Sync + 'static) {
        if let BackendInner::Wgpu(renderer) = &self.inner {
            let mut request = DeviceRequest::new(wake);
            request.start(renderer);
            self.device = Some(request);
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub(super) fn ready(&mut self) -> Result<bool, Error> {
        let (Some(request), BackendInner::Wgpu(renderer)) = (&mut self.device, &mut self.inner)
        else {
            return Ok(true);
        };
        if renderer.device_lost() && !request.waiting() {
            tracing::warn!("sigil-app: GPU device lost; requesting a new one");
            request.start(renderer);
        }
        let Some(result) = request.take() else {
            return Ok(!request.waiting());
        };
        match result {
            Ok(parts) => {
                renderer.adopt_device(parts);
                self.adopt_gpu_status();
            }
            Err(err) if self.fallback => {
                let cpu = self.fall_back_to_cpu(&err);
                self.use_cpu(cpu);
            }
            Err(err) => return Err(err),
        }
        Ok(true)
    }

    pub(super) const fn set_transparent(&mut self, transparent: bool) {
        match &mut self.inner {
            BackendInner::Wgpu(r) => r.set_transparent(transparent),
            BackendInner::Cpu(r) => r.set_transparent(transparent),
        }
    }

    pub(super) const fn is_transparent(&self) -> bool {
        match &self.inner {
            BackendInner::Wgpu(r) => r.is_transparent(),
            BackendInner::Cpu(r) => r.is_transparent(),
        }
    }

    pub(super) const fn last_frame_stats(&self) -> FrameStats {
        match &self.inner {
            BackendInner::Wgpu(r) => r.last_frame_stats,
            BackendInner::Cpu(r) => r.stats,
        }
    }

    pub(super) fn create_surface<W>(
        &mut self,
        window: W,
        size: PhysicalSize,
    ) -> Result<RenderSurface, Error>
    where
        W: HasWindowHandle + HasDisplayHandle + Clone + Send + Sync + 'static,
    {
        let attempt = match &mut self.inner {
            BackendInner::Cpu(r) => {
                return Ok(RenderSurface::Cpu(
                    r.create_surface(into_window_handle(window), size)?,
                ));
            }
            BackendInner::Wgpu(r) => r.create_surface(into_window_handle(window.clone()), size),
        };
        match attempt {
            Ok(surface) => {
                self.adopt_gpu_status();
                Ok(RenderSurface::Wgpu(surface))
            }
            Err(err) if self.fallback => {
                let mut cpu = self.fall_back_to_cpu(&err);
                let surface = cpu.create_surface(into_window_handle(window), size)?;
                self.use_cpu(cpu);
                Ok(RenderSurface::Cpu(surface))
            }
            Err(err) => Err(err),
        }
    }

    pub(super) fn resize(
        &mut self,
        surface: &mut RenderSurface,
        size: PhysicalSize,
    ) -> Result<(), Error> {
        match (&mut self.inner, surface) {
            (BackendInner::Wgpu(r), RenderSurface::Wgpu(s)) => r.resize(s, size),
            (BackendInner::Cpu(r), RenderSurface::Cpu(s)) => r.resize(s, size),
            _ => Err(mismatch()),
        }
    }

    pub(super) fn render(
        &mut self,
        surface: &mut RenderSurface,
        scene: &CommandScene,
        clear: Color,
        viewport: (f32, f32),
    ) -> Result<(), Error> {
        match (&mut self.inner, surface) {
            (BackendInner::Wgpu(r), RenderSurface::Wgpu(s)) => {
                let lost = r.device_lost();
                let result = r.render_command_scene(s, scene, clear, viewport);
                if lost && result.is_ok() {
                    self.adopt_gpu_status();
                }
                result
            }
            (BackendInner::Cpu(r), RenderSurface::Cpu(s)) => {
                r.render_command_scene(s, scene, clear, viewport)
            }
            _ => Err(mismatch()),
        }
    }
}

const fn mismatch() -> Error {
    Error::fixed(
        ErrorKind::Software,
        "software renderer failed: surface was created by a different backend",
    )
}

#[cfg(feature = "inspect")]
impl Backend {
    pub(in crate::app) fn render_image(
        &mut self,
        scene: &CommandScene,
        target: ImageTarget,
    ) -> Result<image::RgbaImage, Error> {
        let (attempt, had_device) = match &mut self.inner {
            BackendInner::Cpu(r) => return render_to_image(ImageRenderer::Cpu(r), scene, target),
            BackendInner::Wgpu(r) => {
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
                self.use_cpu(cpu);
                image
            }
            Err(err) => Err(err),
        }
    }
}

#[cfg(target_arch = "wasm32")]
type DeviceSlot =
    std::rc::Rc<std::cell::RefCell<Option<Result<crate::renderer::DeviceParts, Error>>>>;

#[cfg(target_arch = "wasm32")]
struct DeviceRequest {
    slot: DeviceSlot,
    wake: crate::renderer::OnLost,
    waiting: bool,
}

#[cfg(target_arch = "wasm32")]
impl DeviceRequest {
    fn new(wake: impl Fn() + Send + Sync + 'static) -> Self {
        Self {
            slot: DeviceSlot::default(),
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

    fn take(&mut self) -> Option<Result<crate::renderer::DeviceParts, Error>> {
        let result = self.slot.borrow_mut().take()?;
        self.waiting = false;
        Some(result)
    }
}

#[cfg(all(test, feature = "inspect"))]
#[path = "backend_tests.rs"]
mod tests;
