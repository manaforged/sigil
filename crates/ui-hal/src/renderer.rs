//! Renderer trait — see [DR-001].
//!
//! The `Renderer` trait abstracts GPU backends. In production it is
//! implemented by `ui-renderer` over `wgpu`. Tests implement a headless
//! CPU backend. Consumers name the trait, never the backend crate.
//!
//! # Phase 2 shape
//!
//! The minimum useful surface for `hello-rect`: create a surface bound
//! to a window, resize it, and clear it to a colour. Phase 3 adds a
//! scene recording API (draw quads, text runs, clip stacks) once
//! `ui-core` has a scene graph to encode.
//!
//! [DR-001]: ../../../docs/decisions/DR-001-renderer-wgpu.md
//!
//! # Window handle abstraction
//!
//! [`Renderer::WindowHandle`] is deliberately an associated type. The
//! canonical production binding uses the
//! [`raw_window_handle`][raw-window-handle] traits — the same shape
//! `wgpu::Instance::create_surface` consumes — so `ui-renderer` can take
//! a boxed `dyn HasWindowHandle + HasDisplayHandle + Send + Sync` and
//! hand it to wgpu without a second abstraction. Test adapters can pick
//! a different handle type (e.g. `()` for headless CPU renderers) since
//! the HAL only requires that consumers produce whatever the backend
//! asks for.
//!
//! The trait-object indirection is paid once, at surface creation, not
//! on any hot path.
//!
//! [raw-window-handle]: https://docs.rs/raw-window-handle

use core::error::Error;

use crate::geometry::{Color, PhysicalSize};

/// GPU backend abstraction.
///
/// See the module-level docs for the decision record behind this shape.
pub trait Renderer {
    /// Error type returned by all fallible operations.
    type Error: Error + Send + Sync + 'static;

    /// Opaque handle to a rendering surface bound to a window.
    ///
    /// The concrete type is owned by the implementation. `ui-renderer`
    /// returns a `WgpuSurface`; a headless test renderer might return a
    /// `HeadlessSurface` with CPU-side pixel storage.
    type Surface;

    /// Adapter-specific window binding type consumed by
    /// [`Renderer::create_surface`].
    ///
    /// `ui-renderer` uses a boxed `raw_window_handle` trait object so it
    /// can back any winit / SDL / native window without coupling to a
    /// specific windowing crate. Test renderers can use `()` or a
    /// lightweight stub.
    type WindowHandle;

    /// Create a new GPU surface bound to a window.
    ///
    /// `initial_size` is the window's physical size at creation time,
    /// used to configure the swapchain. Zero-sized dimensions are
    /// clamped internally so the call never fails for a freshly-created
    /// window that has not yet produced a non-zero size event.
    fn create_surface(
        &mut self,
        window: Self::WindowHandle,
        initial_size: PhysicalSize,
    ) -> Result<Self::Surface, Self::Error>;

    /// Resize the backing surface after a window resize.
    ///
    /// A zero-sized resize must be a no-op so backends can ignore
    /// minimise events without special-casing at call sites.
    fn resize(
        &mut self,
        surface: &mut Self::Surface,
        size: PhysicalSize,
    ) -> Result<(), Self::Error>;

    /// Render a single frame whose only content is a full-surface clear
    /// to `color`, then present.
    ///
    /// Phase 2 gate primitive — Phase 3 replaces this with a scene
    /// recording API. For now, clear-and-present is sufficient to prove
    /// the backend pipeline is alive end-to-end.
    ///
    /// Implementations must recover transparently from surface-lost and
    /// surface-outdated conditions (typically by reconfiguring and
    /// skipping the frame) rather than surfacing them as errors, since
    /// those states are routine during window resize and monitor
    /// reconfiguration.
    fn clear(&mut self, surface: &mut Self::Surface, color: Color) -> Result<(), Self::Error>;
}
