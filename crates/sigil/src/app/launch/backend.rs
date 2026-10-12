use crate::geometry::PhysicalSize;
use crate::hal::Renderer as _;
use crate::profile::{BackendKind, BackendStatus, FallbackCause};
use crate::renderer::frame_stats::FrameStats;
use crate::renderer::{
    CpuRenderer, CpuSurface, RendererError, WgpuRenderer, WgpuSurface, into_window_handle,
};
use crate::scene::CommandScene;
use crate::style::Color;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};

#[cfg(feature = "inspect")]
mod inspect;
#[cfg(any(feature = "inspect", not(target_arch = "wasm32")))]
mod offscreen;

#[cfg(feature = "inspect")]
pub(super) use inspect::inspect_shot;

const ENV: &str = "SIGIL_BACKEND";
const SURFACE_CLEAR: Color = Color::rgba(0.03, 0.04, 0.05, 1.0);
const TRANSPARENT_CLEAR: Color = Color::rgba(0.0, 0.0, 0.0, 0.0);

pub(in crate::app) struct Backend {
    inner: Inner,
    fallback: bool,
    status: BackendStatus,
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
