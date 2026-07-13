//! Shell trait — see [DR-002].
//!
//! The `Shell` trait abstracts the OS window and event loop. In
//! production this is implemented over `winit` 0.30+ by `sigil-shell`.
//! Tests implement a headless shell that feeds synthetic events.
//!
//! The shell is the only part of the library that knows about the OS
//! event loop. Signal reads and writes happen on the shell's main
//! thread; background work funnels through the [`Scheduler`] into the
//! main-thread dispatcher and wakes the shell via [`Shell::wake`].
//!
//! # Event delivery shape
//!
//! This trait is deliberately narrow. It encodes only:
//!
//! 1. **Window lifecycle** — `create_window` queues a window spec; the
//!    adapter materialises it when the OS event loop is ready
//!    (`ApplicationHandler::resumed` in the winit implementation).
//! 2. **Cross-thread wake** — a `Waker` handle is `Clone + Send + Sync`
//!    and drives the coalescing wake documented in DR-002/DR-007.
//! 3. **Event drain** — platform adapters translate OS input and
//!    lifecycle notifications into [`ShellEvent`] values, then expose
//!    them through [`Shell::drain_events`] during the frame callback.
//! 4. **Event-loop entry point** — `run` consumes the shell, drives
//!    the OS event loop, and invokes the user's `frame` callback at
//!    redraw time with a mutable handle to the shell.
//!
//! Routing, hit-testing, focus management, and handler dispatch live
//! in `sigil-events`. `Shell` only owns platform delivery.
//!
//! [DR-002]: ../../../docs/decisions/DR-002-shell-winit.md
//! [`Scheduler`]: crate::scheduler::Scheduler

use core::error::Error;

use alloc::vec::IntoIter;

use crate::{
    ShellEvent,
    geometry::{PhysicalSize, Rect},
    size::{LogicalSize, Scale},
};

/// OS window and event loop abstraction.
///
/// See the module-level documentation for the scope and lifecycle
/// guarantees this trait is responsible for.
pub trait Shell: Sized {
    /// Error type returned by shell operations.
    type Error: Error + Send + Sync + 'static;

    /// Handle used to wake the event loop from another thread.
    ///
    /// Implementations must coalesce wakes — multiple calls before the
    /// next event-loop iteration collapse to one delivered wake.
    /// Payload travels through the [`Scheduler`]'s MPSC channel, not
    /// through the wake handle.
    ///
    /// [`Scheduler`]: crate::scheduler::Scheduler
    type Waker: Clone + Send + Sync + 'static;

    /// Opaque handle representing a window that has been requested.
    ///
    /// The handle is returned immediately by [`Self::create_window`],
    /// but the underlying OS window may not exist yet — winit forbids
    /// window construction outside the `resumed` callback. Consumers
    /// use the handle only as an identity token; concrete accessors
    /// live on the adapter type, not on the trait.
    type Window;

    /// Return a wake handle that can be cloned to background threads.
    fn waker(&self) -> Self::Waker;

    /// Wake the event loop.
    ///
    /// Conceptually equivalent to `winit::EventLoopProxy::wake_up()`
    /// (in winit 0.30 this is surfaced as `send_event`). Provided as a
    /// free function on the waker so that tests — and `sigil-async` — can
    /// hold a `Waker` without a live `Shell` reference.
    /// # Errors
    ///
    /// Returns an error when the platform event loop can no longer be
    /// woken through this handle.
    fn wake(waker: &Self::Waker) -> Result<(), Self::Error>;

    /// Request creation of a new window.
    ///
    /// The spec is queued; the OS window is created when the event
    /// loop is ready. In the winit adapter that means inside
    /// `ApplicationHandler::resumed`. Callers receive a handle
    /// immediately so they can wire up dependent state (renderer,
    /// waker, etc.) without waiting for the event loop to start.
    /// # Errors
    ///
    /// Returns an error when the shell cannot queue or create the
    /// requested window.
    fn create_window(
        &mut self,
        title: &str,
        initial_size: PhysicalSize,
    ) -> Result<Self::Window, Self::Error>;

    /// Create a window sized in logical pixels.
    ///
    /// The shell adapter is responsible for converting to the
    /// destination monitor's physical pixels using its current scale
    /// factor, so the resulting OS window matches the user's requested
    /// logical dimensions on high-density displays.
    ///
    /// The default implementation preserves compatibility for adapters
    /// that only understand physical pixels.
    /// # Errors
    ///
    /// Returns an error when the shell cannot queue or create the
    /// requested window.
    fn create_window_logical(
        &mut self,
        title: &str,
        initial: LogicalSize,
    ) -> Result<Self::Window, Self::Error> {
        self.create_window(title, Scale::ONE.to_physical(initial))
    }

    /// Drain input and lifecycle events buffered since the last frame.
    ///
    /// The returned iterator owns the drained buffer, mirroring
    /// `sigil-async`'s main-thread drain shape.
    fn drain_events(&mut self) -> IntoIter<ShellEvent>;

    /// Run the event loop until exit.
    ///
    /// Consumes the shell. At each redraw opportunity the
    /// `frame` callback is invoked with a mutable handle to the shell
    /// so it can access its windows, request new ones, or trigger
    /// further redraws. Returns when the event loop exits (e.g. the
    /// last window closes).
    ///
    /// Event loop orchestration is a trait method rather than a
    /// concrete method on the adapter so that headless test shells
    /// can stand in for the real winit shell behind the same surface.
    /// # Errors
    ///
    /// Returns an error when the platform event loop exits with a
    /// backend failure instead of a normal application shutdown.
    fn run<F>(self, frame: F) -> Result<(), Self::Error>
    where
        F: FnMut(&mut Self) + 'static;

    /// Toggle whether the OS IME is allowed for `window`. Drives
    /// focus-change-based enable/disable — the application calls this
    /// when focus enters/leaves an IME-targeting widget so IME
    /// candidate UI only appears over a composition-aware field. See
    /// DR-051 §4.8.
    ///
    /// Default is a no-op so headless/test shells don't need an impl.
    ///
    /// # Errors
    ///
    /// Returns an error when the platform reports an IME toggle
    /// failure (rare; most adapters can't fail this call).
    fn set_ime_allowed(&mut self, window: &Self::Window, allowed: bool) -> Result<(), Self::Error> {
        let _ = (window, allowed);
        Ok(())
    }

    /// Forward the candidate-window anchor rect to the OS IME. `rect`
    /// is window-relative logical pixels; the shell adapter converts
    /// to whatever coordinate space its backend needs. See DR-051
    /// §4.8 / §Decision-4.
    ///
    /// Default is a no-op so headless/test shells don't need an impl.
    ///
    /// # Errors
    ///
    /// Returns an error when the platform rejects the rect.
    fn set_ime_cursor_area(
        &mut self,
        window: &Self::Window,
        rect: Rect,
    ) -> Result<(), Self::Error> {
        let _ = (window, rect);
        Ok(())
    }
}
