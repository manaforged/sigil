//! The opaque `WindowHandle` binding consumed by [`WgpuRenderer`].
//!
//! We use `raw-window-handle` rather than coupling to `winit` because
//! that is the canonical shape `wgpu::Instance::create_surface` itself
//! consumes — binding to it here avoids a second abstraction layer and
//! keeps the HAL genuinely vendor-agnostic (see
//! `crates/sigil/hal/src/renderer.rs` for the full decision record).
//!
//! The trait-object indirection is paid exactly once, inside
//! [`Renderer::create_surface`][sigil_hal::Renderer::create_surface]. No
//! hot path crosses this boundary.

use raw_window_handle::{HasDisplayHandle, HasWindowHandle};

/// Opaque window binding accepted by
/// [`WgpuRenderer`][crate::WgpuRenderer].
///
/// Constructed via [`into_window_handle`] from any value that satisfies
/// the `raw-window-handle` traits plus `Send + Sync + 'static`. That
/// matches `Arc<winit::window::Window>` (the primary production target)
/// and the ergonomic desktop path without requiring `sigil-renderer` to
/// name `winit` directly.
pub struct WindowHandle {
    inner: Box<dyn DisplayAndWindow>,
}

impl WindowHandle {
    pub(crate) fn into_inner(self) -> Box<dyn DisplayAndWindow> {
        self.inner
    }
}

/// Wrap any `Send + Sync + 'static` raw-window-handle producer in a
/// [`WindowHandle`].
///
/// This is the only intended constructor. Keeping construction behind a
/// free function (rather than `From` / `new`) makes the
/// `raw-window-handle` dependency explicit at the call site.
pub fn into_window_handle<W>(window: W) -> WindowHandle
where
    W: HasWindowHandle + HasDisplayHandle + Send + Sync + 'static,
{
    WindowHandle {
        inner: Box::new(window),
    }
}

/// Crate-private supertrait collapsing the `raw-window-handle`
/// producer bounds into a single object-safe trait we can store in a
/// `Box<dyn>`. Visibility is `pub(crate)` so the `renderer` module can
/// receive the box; the whole module is private so nothing leaks past
/// the crate boundary.
#[allow(
    clippy::redundant_pub_crate,
    reason = "pub(crate) is load-bearing: DisplayAndWindow shows up in \
              WindowHandle::into_inner's signature which is consumed by \
              the sibling `renderer` module."
)]
pub(crate) trait DisplayAndWindow: HasWindowHandle + HasDisplayHandle + Send + Sync {}

impl<T> DisplayAndWindow for T where T: HasWindowHandle + HasDisplayHandle + Send + Sync {}
