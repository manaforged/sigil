//! winit 0.30+ `ApplicationHandler`-backed Shell adapter.
//!
//! Implements [`ui_hal::Shell`] over `winit` 0.30+. See
//! [DR-002](../../../docs/decisions/DR-002-shell-winit.md) for the
//! load-bearing invariants.
//!
//! # Lifecycle
//!
//! `WinitShell::new()` constructs an `EventLoop<WakeEvent>` eagerly so
//! that the [`Waker`] can be handed out before the event loop is
//! running. Window specifications submitted via
//! [`ui_hal::Shell::create_window`] are queued until winit fires
//! `resumed`, at which point the adapter creates them on the active
//! event loop. The adapter also drains the pending queue on each
//! `user_event` tick and at the top of every `RedrawRequested`, so
//! frame-time window requests resolve on the next event-loop iteration
//! rather than sitting un-materialised forever.
//!
//! [`Waker`]: ui_hal::Shell::Waker
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
//! ## Coalescing contract
//!
//! The HAL contract (see [`ui_hal::Shell::Waker`]) requires that
//! multiple `wake()` calls before the next event-loop iteration
//! collapse into a single delivered wake. We honour that with an
//! `Arc<AtomicBool>` "pending wake" flag shared between the waker and
//! the application handler:
//!
//! - `WinitWaker::wake` attempts a `compare_exchange(false, true,
//!   Acquire, Relaxed)`. Only a false→true transition actually forwards
//!   a `WakeEvent` through the proxy — subsequent calls observe the
//!   flag already set and return `Ok(())` without touching the proxy.
//! - Inside `ApplicationHandler::user_event`, after the wake hook has
//!   drained the ui-async MPSC, we store `false` with `Release`
//!   ordering. This pairs with the `Acquire` on the next wake: any
//!   enqueue that happens between `user_event` returning and the next
//!   successful compare-exchange is guaranteed to be visible when the
//!   hook runs.
//!
//! A 100-task fan-out therefore produces at most one queued
//! `WakeEvent` between two event-loop iterations, regardless of how
//! many background tasks called `wake()`.
//!
//! # ui-async integration
//!
//! `ui-async` needs a way to wake the main thread from background
//! Tokio tasks. The canonical bridge is:
//!
//! 1. The app constructs `WinitShell`.
//! 2. It calls `shell.waker()` to get a cloneable, thread-safe handle.
//! 3. It passes that handle (or a thin closure over it) into the
//!    `ui-async` runtime constructor.
//! 4. Background tasks push `SignalUpdate` payloads into `ui-async`'s
//!    MPSC and call the waker — `WinitWaker::wake()` or the free
//!    function `<WinitShell as Shell>::wake(&waker)` — to fire one
//!    coalesced `WakeEvent` into the event loop.
//! 5. Inside `ApplicationHandler::user_event`, the adapter notifies
//!    the main-thread dispatcher so it can drain the MPSC before the
//!    next frame.
//!
//! The drain hook is wired via [`WinitShell::on_wake`], which takes an
//! `Arc<dyn Fn() + Send + Sync + 'static>`. This lets us ship the
//! adapter before `ui-async` stabilises its public API — the bridge
//! is a plain closure, not a typed runtime handle. When `ui-async`
//! lands, consumers will simply hand it a closure that calls
//! `AsyncRuntime::drain`.
//!
//! TODO: wire a typed `ui_async::AsyncRuntime` handle when its API
//! stabilises.

use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicU64, Ordering},
};

use ui_hal::{PhysicalSize, Shell};
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize as WinitPhysicalSize,
    error::EventLoopError,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy},
    window::{Window, WindowAttributes, WindowId},
};

/// Test-only counter that increments every time [`WinitWaker::wake`]
/// transitions the coalescing flag from `false` to `true` and
/// therefore actually forwards a `WakeEvent` through the proxy.
///
/// Exposed as `#[doc(hidden)] pub` (instead of gated on `cfg(test)`)
/// so that integration tests in `tests/shell_contract.rs` — which
/// compile against the library as an external crate — can observe it.
/// The static is public API only in the strictly technical sense; it
/// is not part of the supported surface and must not be relied on by
/// consumers.
#[doc(hidden)]
pub static WAKE_DELIVERED_COUNT: AtomicU64 = AtomicU64::new(0);

/// The single user-event payload the adapter sends through
/// `EventLoopProxy::send_event`.
///
/// Kept as a unit struct on purpose: the proxy coalesces *intent*
/// (one or more `WakeEvent`s collapse into one delivered `user_event`
/// callback), and the payload — `SignalUpdate`s enqueued by
/// `ui-async` — travels through an MPSC channel rather than through
/// the event loop. See DR-002 and DR-007.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Default)]
pub struct WakeEvent;

/// Cloneable, thread-safe wake handle for [`WinitShell`].
///
/// Produced by [`WinitShell::waker`] and consumed by `ui-async` (or
/// anything else that needs to signal the main thread). Wrapping the
/// winit proxy in `Arc<Mutex<..>>` keeps the handle `Send + Sync`
/// without using `unsafe`; the mutex is uncontended in practice.
///
/// The waker coalesces wakes: see the module-level "Coalescing
/// contract" section for details.
#[derive(Clone)]
pub struct WinitWaker {
    proxy: Arc<Mutex<EventLoopProxy<WakeEvent>>>,
    pending: Arc<AtomicBool>,
    alive: Arc<AtomicBool>,
}

impl WinitWaker {
    /// Signal the event loop.
    ///
    /// Coalesces: only the `false`→`true` transition of the shared
    /// pending flag actually forwards a `WakeEvent` through the
    /// proxy. Subsequent calls before the next `user_event` tick
    /// return `Ok(())` without touching the proxy. See the module
    /// docs for the ordering rationale.
    ///
    /// Returns an error if the event loop has already exited. The
    /// liveness check is performed before the coalescing CAS so that
    /// a dead loop is always surfaced — even if a stale pending bit
    /// is set.
    pub fn wake(&self) -> Result<(), ShellError> {
        // Liveness gate first: the shell flips `alive` to false in
        // its `Drop` impl. Without this check, a stale pending bit
        // could mask a dead event loop indefinitely.
        if !self.alive.load(Ordering::Acquire) {
            return Err(ShellError::EventLoopClosed);
        }

        // Acquire on success pairs with the Release store in
        // `user_event` after the wake hook has drained: any MPSC
        // enqueue that happens-before our CAS is visible to the hook
        // that will process the wake we're about to deliver.
        if self
            .pending
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            // Another call already scheduled a wake that has not yet
            // been delivered; collapse into it.
            return Ok(());
        }

        let proxy = self.proxy.lock().map_err(|_| ShellError::WakerPoisoned)?;
        if proxy.send_event(WakeEvent).is_ok() {
            WAKE_DELIVERED_COUNT.fetch_add(1, Ordering::Relaxed);
            Ok(())
        } else {
            // The event loop died between the liveness check and
            // the send (or never saw the alive flag flip). Reset
            // the pending flag so future calls don't silently
            // coalesce, and surface the error by flipping the
            // alive flag so subsequent wakes short-circuit.
            self.pending.store(false, Ordering::Release);
            self.alive.store(false, Ordering::Release);
            Err(ShellError::EventLoopClosed)
        }
    }
}

impl core::fmt::Debug for WinitWaker {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("WinitWaker").finish_non_exhaustive()
    }
}

/// Handle for a window requested via [`ui_hal::Shell::create_window`].
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
    /// [`ui_hal::Shell::run`] was called but the event loop had
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

/// Pending window specification recorded before the event loop is
/// alive, or between frames.
struct PendingWindow {
    handle: WinitWindowHandle,
    title: String,
    size: PhysicalSize,
}

/// Closure invoked inside `ApplicationHandler::user_event`, used by
/// `ui-async` to drain its MPSC.
type WakeHook = Arc<dyn Fn() + Send + Sync + 'static>;

/// winit-backed [`Shell`] implementation.
pub struct WinitShell {
    event_loop: Option<EventLoop<WakeEvent>>,
    proxy: Arc<Mutex<EventLoopProxy<WakeEvent>>>,
    pending_wake: Arc<AtomicBool>,
    alive: Arc<AtomicBool>,
    pending: Vec<PendingWindow>,
    windows: Vec<(WinitWindowHandle, Arc<Window>)>,
    failed: Vec<(WinitWindowHandle, String)>,
    next_handle: u64,
    wake_hook: Option<WakeHook>,
}

impl Drop for WinitShell {
    fn drop(&mut self) {
        // Flip the liveness flag before the event loop is dropped so
        // any waker call that races with teardown observes the dead
        // loop and surfaces `EventLoopClosed` instead of silently
        // coalescing against a stale pending bit.
        self.alive.store(false, Ordering::Release);
    }
}

impl WinitShell {
    /// Construct a new shell.
    ///
    /// Builds a `winit::event_loop::EventLoop<WakeEvent>` eagerly so
    /// that [`Self::waker`] can be called — and its output handed to
    /// `ui-async` — before [`Shell::run`] is called.
    pub fn new() -> Result<Self, ShellError> {
        let event_loop = EventLoop::<WakeEvent>::with_user_event().build()?;
        let proxy = Arc::new(Mutex::new(event_loop.create_proxy()));
        Ok(Self {
            event_loop: Some(event_loop),
            proxy,
            pending_wake: Arc::new(AtomicBool::new(false)),
            alive: Arc::new(AtomicBool::new(true)),
            pending: Vec::new(),
            windows: Vec::new(),
            failed: Vec::new(),
            next_handle: 0,
            wake_hook: None,
        })
    }

    /// Register a closure that is invoked once per delivered
    /// `WakeEvent` inside `ApplicationHandler::user_event`.
    ///
    /// This is the bridge `ui-async` will use to drain its MPSC at
    /// the start of each frame. The hook is stored, not moved into
    /// the running event loop, so it can be installed at any time
    /// before [`Shell::run`].
    pub fn on_wake<F>(&mut self, hook: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.wake_hook = Some(Arc::new(hook));
    }

    /// Return the live window for a handle, if it has been
    /// materialised yet.
    ///
    /// Windows are created lazily in `ApplicationHandler::resumed`
    /// and on every `user_event` / `RedrawRequested` tick; before the
    /// first such tick this returns `None`.
    #[must_use]
    pub fn window_for(&self, handle: WinitWindowHandle) -> Option<&Arc<Window>> {
        self.windows
            .iter()
            .find_map(|(h, w)| (*h == handle).then_some(w))
    }

    /// Return every live window.
    #[must_use]
    pub fn windows(&self) -> &[(WinitWindowHandle, Arc<Window>)] {
        &self.windows
    }

    /// Drain the list of window specifications that failed to
    /// materialise, together with the stringified `winit` error.
    ///
    /// The adapter keeps going when a single `create_window` call
    /// fails at materialisation time so one bad spec does not doom
    /// the app. Callers that care about surfacing the failure — tests,
    /// error-reporting UI — pull it out here. Each entry is returned
    /// at most once.
    #[must_use]
    pub fn take_failed_windows(&mut self) -> Vec<(WinitWindowHandle, String)> {
        std::mem::take(&mut self.failed)
    }

    const fn allocate_handle(&mut self) -> WinitWindowHandle {
        let id = self.next_handle;
        self.next_handle = self.next_handle.wrapping_add(1);
        WinitWindowHandle(id)
    }

    fn materialise_pending(&mut self, active: &ActiveEventLoop) {
        if self.pending.is_empty() {
            return;
        }
        // Drain into a temporary vec to avoid borrow conflicts.
        let pending = std::mem::take(&mut self.pending);
        for spec in pending {
            let attrs = WindowAttributes::default()
                .with_title(spec.title)
                .with_inner_size(WinitPhysicalSize::new(spec.size.width, spec.size.height));
            match active.create_window(attrs) {
                Ok(window) => self.windows.push((spec.handle, Arc::new(window))),
                Err(err) => {
                    // Record the failure so callers can surface it via
                    // `take_failed_windows`. A real logging integration
                    // lands with the observability pass; until then the
                    // structured list is the single source of truth.
                    self.failed.push((spec.handle, err.to_string()));
                }
            }
        }
    }
}

impl Shell for WinitShell {
    type Error = ShellError;
    type Waker = WinitWaker;
    type Window = WinitWindowHandle;

    fn waker(&self) -> Self::Waker {
        WinitWaker {
            proxy: Arc::clone(&self.proxy),
            pending: Arc::clone(&self.pending_wake),
            alive: Arc::clone(&self.alive),
        }
    }

    fn wake(waker: &Self::Waker) -> Result<(), Self::Error> {
        waker.wake()
    }

    fn create_window(
        &mut self,
        title: &str,
        initial_size: PhysicalSize,
    ) -> Result<Self::Window, Self::Error> {
        let handle = self.allocate_handle();
        self.pending.push(PendingWindow {
            handle,
            title: title.to_owned(),
            size: initial_size,
        });
        Ok(handle)
    }

    fn run<F>(mut self, frame: F) -> Result<(), Self::Error>
    where
        F: FnMut(&mut Self) + 'static,
    {
        let event_loop = self.event_loop.take().ok_or(ShellError::AlreadyRun)?;
        let mut app = WinitApp { shell: self, frame };
        event_loop.run_app(&mut app)?;
        Ok(())
    }
}

struct WinitApp<F> {
    shell: WinitShell,
    frame: F,
}

impl<F> ApplicationHandler<WakeEvent> for WinitApp<F>
where
    F: FnMut(&mut WinitShell),
{
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.shell.materialise_pending(event_loop);
        for (_, window) in &self.shell.windows {
            window.request_redraw();
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, _event: WakeEvent) {
        // Materialise any windows requested between ticks so that
        // frame-time `create_window` calls resolve on the next event
        // loop iteration rather than hanging forever.
        self.shell.materialise_pending(event_loop);

        if let Some(hook) = self.shell.wake_hook.as_ref() {
            hook();
        }

        // Release pairs with the Acquire on the waker's
        // compare_exchange. After this store, subsequent wake() calls
        // can schedule a fresh delivery — but the hook has already
        // observed any writes that happened-before those calls.
        self.shell.pending_wake.store(false, Ordering::Release);

        // Wake hook drained the MPSC; trigger a repaint so any
        // signal-driven updates land in the next frame.
        for (_, window) in &self.shell.windows {
            window.request_redraw();
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::RedrawRequested => {
                // Drain any late-arriving window specs so frame-time
                // creation calls complete before the frame callback
                // runs.
                self.shell.materialise_pending(event_loop);
                (self.frame)(&mut self.shell);
            }
            _ => {}
        }
    }
}
