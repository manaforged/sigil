use crate::hal::{Color, PhysicalSize, Renderer as _};
use crate::renderer::{
    CommandScene, CpuRenderer, CpuSurface, FrameStats, RendererError, WgpuRenderer, WgpuSurface,
    into_window_handle,
};
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};

const ENV: &str = "SIGIL_BACKEND";

pub(super) struct Backend {
    inner: Inner,
    fallback: bool,
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
            }),
            "wgpu" | "gpu" => Ok(Self {
                inner: Inner::Wgpu(Box::new(WgpuRenderer::new()?)),
                fallback: false,
            }),
            "" | "auto" => Ok(Self {
                inner: Inner::Wgpu(Box::new(WgpuRenderer::new()?)),
                fallback: true,
            }),
            other => {
                tracing::warn!("sigil-app: {ENV}={other:?} is not cpu, wgpu, or auto; using auto");
                Ok(Self {
                    inner: Inner::Wgpu(Box::new(WgpuRenderer::new()?)),
                    fallback: true,
                })
            }
        }
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
}

fn mismatch() -> RendererError {
    RendererError::Software("surface was created by a different backend".to_owned())
}
