//! winit 0.30+ `ApplicationHandler`-backed Shell adapter.
//!
//! Implements [`sigil_hal::Shell`] over `winit` 0.30+. See
//! [DR-002](../../../docs/decisions/DR-002-shell-winit.md) for the
//! load-bearing invariants.
//!
//! # Lifecycle
//!
//! [`WinitShell::new`] constructs an `EventLoop<WakeEvent>` eagerly so
//! that the [`Waker`] can be handed out before the event loop is
//! running. Window specifications submitted via
//! [`sigil_hal::Shell::create_window`] are queued until winit fires
//! `resumed`, at which point the adapter creates them on the active
//! event loop. The adapter also drains the pending queue on each
//! `user_event` tick and at the top of every `RedrawRequested`, so
//! frame-time window requests resolve on the next event-loop iteration
//! rather than sitting un-materialised forever.
//!
//! [`Waker`]: sigil_hal::Shell::Waker
//!
//! # Wake bridge
//!
//! The adapter exposes a [`WinitWaker`] that wraps
//! `Arc<Mutex<EventLoopProxy<WakeEvent>>>`. The mutex is not there for
//! mutual exclusion — `EventLoopProxy::send_event` takes `&self` and
//! the platform implementations are internally thread-safe — it is
//! there to lift `Sync` uniformly across platforms. `EventLoopProxy`
//! is only marked `Sync` on macOS/iOS in winit 0.30.13, so wrapping it
//! in a `Mutex` gives us a cross-platform `Send + Sync` handle without
//! any `unsafe` (the workspace denies `unsafe_code`).
//!
//! The `wake()` path coalesces multiple calls into one delivered
//! `WakeEvent` between event-loop iterations. See the [`waker`] module
//! for the ordering rationale.
//!
//! # sigil-async integration
//!
//! `sigil-async` needs a way to wake the main thread from background
//! Tokio tasks. The canonical bridge is:
//!
//! 1. The app constructs [`WinitShell`].
//! 2. It calls `shell.waker()` to get a cloneable, thread-safe handle.
//! 3. It passes that handle (or a thin closure over it) into the
//!    `sigil-async` runtime constructor.
//! 4. Background tasks push `SignalUpdate` payloads into `sigil-async`'s
//!    MPSC and call the waker — [`WinitWaker::wake`] or the free
//!    function `<WinitShell as Shell>::wake(&waker)` — to fire one
//!    coalesced [`WakeEvent`] into the event loop.
//! 5. Inside `ApplicationHandler::user_event`, the adapter notifies
//!    the main-thread dispatcher so it can drain the MPSC before the
//!    next frame.
//!
//! The drain hook is wired via [`WinitShell::on_wake`], which takes an
//! `Arc<dyn Fn() + Send + Sync + 'static>`. This lets us ship the
//! adapter before `sigil-async` stabilises its public API — the bridge
//! is a plain closure, not a typed runtime handle.
//!
//! TODO: wire a typed `sigil_async::AsyncRuntime` handle when its API
//! stabilises.

use winit::error::EventLoopError;

pub(crate) mod app;
pub(crate) mod dpi;
pub(crate) mod event_map;
pub(crate) mod key_map;
pub(crate) mod shell;
pub(crate) mod waker;

pub use shell::WinitShell;
pub use waker::{WAKE_DELIVERED_COUNT, WakeEvent, WinitWaker};

/// Handle for a window requested via [`sigil_hal::Shell::create_window`].
///
/// The handle is returned immediately, but the underlying OS window
/// is only created once winit fires `resumed`. Use
/// [`WinitShell::window_for`] during the `run` frame callback to look
/// up the live `Arc<Window>`.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct WinitWindowHandle(u64);

impl WinitWindowHandle {
    /// Raw handle id. Exposed for logging and test assertions.
    #[must_use]
    pub const fn id(self) -> u64 {
        self.0
    }

    pub(crate) const fn from_raw(id: u64) -> Self {
        Self(id)
    }
}

/// Errors surfaced by the adapter.
#[derive(Debug)]
pub enum ShellError {
    /// The underlying `winit::event_loop::EventLoop` failed to
    /// construct or to run.
    EventLoop(EventLoopError),
    /// `send_event` was called after the event loop exited.
    EventLoopClosed,
    /// The waker's internal mutex was poisoned.
    WakerPoisoned,
    /// [`sigil_hal::Shell::run`] was called but the event loop had
    /// already been consumed by a previous call.
    AlreadyRun,
}

impl core::fmt::Display for ShellError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::EventLoop(err) => write!(f, "winit event loop error: {err}"),
            Self::EventLoopClosed => f.write_str("event loop already exited"),
            Self::WakerPoisoned => f.write_str("waker mutex poisoned"),
            Self::AlreadyRun => f.write_str("shell run called twice"),
        }
    }
}

impl std::error::Error for ShellError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::EventLoop(err) => Some(err),
            _ => None,
        }
    }
}

impl From<EventLoopError> for ShellError {
    fn from(err: EventLoopError) -> Self {
        Self::EventLoop(err)
    }
}
