//! `WinitShell` — the winit-backed [`hal::Shell`] implementation.
//! See DR-002 for the lifecycle contract: windows are queued until the
//! event loop is alive, and the event loop is built eagerly so the
//! [`Shell::waker`] handle can be handed to `sigil-async` before
//! [`Shell::run`] is called.

#![allow(clippy::redundant_pub_crate)]

use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::time::Instant;

use hal::{
    LogicalSize, ModifiersState, MouseButtons, PhysicalSize, Point, PointerPhase, Scale, Shell,
    ShellEvent, WindowEvent as SigilWindowEvent,
};
use winit::{
    dpi::{LogicalSize as WinitLogicalSize, PhysicalSize as WinitPhysicalSize, Size as WinitSize},
    event::WindowEvent as WinitWindowEvent,
    event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy},
    window::{Window, WindowAttributes, WindowId},
};

use crate::{
    ShellError, WinitWindowHandle,
    app::WinitApp,
    dpi::{scale_for_window_id, scale_from_f64},
    waker::{WakeEvent, WinitWaker},
};

mod events;

/// Pending window specification recorded before the event loop is
/// alive, or between frames.
pub(crate) struct PendingWindow {
    handle: WinitWindowHandle,
    title: String,
    size: PendingWindowSize,
}

pub(crate) enum PendingWindowSize {
    Physical(PhysicalSize),
    Logical(LogicalSize),
}

/// Closure invoked inside `ApplicationHandler::user_event`, used by
/// `sigil-async` to drain its MPSC.
pub(crate) type WakeHook = Arc<dyn Fn() + Send + Sync + 'static>;

/// Closure consulted between event-loop iterations to discover when
/// the soonest piece of pending work is due.
///
/// Returning `Some(deadline)` causes the adapter to set
/// `ControlFlow::WaitUntil(deadline)`, letting the OS wake the loop at
/// the requested time without a busy spin. Returning `None` falls back
/// to the default `Wait` behaviour. Wired by the application layer
/// (see [`WinitShell::on_idle_deadline`]) to surface
/// `reactive::next_interval_deadline()` to the event loop.
pub(crate) type DeadlineHook = Arc<dyn Fn() -> Option<Instant> + Send + Sync + 'static>;

/// winit-backed [`Shell`] implementation.
pub struct WinitShell {
    event_loop: Option<EventLoop<WakeEvent>>,
    proxy: Arc<Mutex<EventLoopProxy<WakeEvent>>>,
    pending_wake: Arc<AtomicBool>,
    alive: Arc<AtomicBool>,
    pending: Vec<PendingWindow>,
    windows: Vec<(WinitWindowHandle, Arc<Window>)>,
    window_scales: Vec<(WinitWindowHandle, Scale)>,
    failed: Vec<(WinitWindowHandle, String)>,
    events: Vec<ShellEvent>,
    pointer_position: Point,
    pointer_buttons: MouseButtons,
    modifiers: ModifiersState,
    next_handle: u64,
    wake_hook: Option<WakeHook>,
    deadline_hook: Option<DeadlineHook>,
    exit_requested: bool,
    /// Set when a `RedrawRequested` is OS-driven (window first-show, un-occlude,
    /// resize) rather than one we scheduled to coalesce input. The frame loop's
    /// change-gate MUST paint such a frame even when no signal changed — the
    /// drawable needs pixels or it shows an unpainted (gray) buffer. Consumed
    /// once per frame via [`Self::take_force_paint`].
    force_paint: bool,
}

impl Drop for WinitShell {
    fn drop(&mut self) {
        // Flip liveness before the event loop drops so racing wakers
        // surface `EventLoopClosed` instead of coalescing against a
        // stale pending bit.
        self.alive.store(false, Ordering::Release);
    }
}

impl WinitShell {
    /// Construct a new shell, building the winit event loop eagerly
    /// so [`Self::waker`] can be called before [`Shell::run`].
    ///
    /// # Errors
    ///
    /// Returns an error when winit cannot construct the platform event loop.
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
            window_scales: Vec::new(),
            failed: Vec::new(),
            events: Vec::new(),
            pointer_position: Point::new(0.0, 0.0),
            pointer_buttons: MouseButtons::empty(),
            modifiers: ModifiersState::empty(),
            next_handle: 0,
            wake_hook: None,
            deadline_hook: None,
            exit_requested: false,
            force_paint: false,
        })
    }

    /// Register a closure invoked once per delivered `WakeEvent`
    /// inside `ApplicationHandler::user_event`.
    pub fn on_wake<F: Fn() + Send + Sync + 'static>(&mut self, hook: F) {
        self.wake_hook = Some(Arc::new(hook));
    }

    /// Register a closure consulted between event-loop iterations to
    /// discover the soonest pending deadline (e.g. the next
    /// [`reactive::interval`](https://docs.rs/sigil-reactive) fire).
    ///
    /// When the hook returns `Some(deadline)`, the adapter sets winit's
    /// `ControlFlow::WaitUntil(deadline)` so the OS wakes the loop at
    /// the requested time without busy-spinning. Returning `None` falls
    /// back to the default `Wait` behaviour. Apps wire this to
    /// `reactive::next_interval_deadline` from inside their
    /// frame-driver setup.
    pub fn on_idle_deadline<F: Fn() -> Option<Instant> + Send + Sync + 'static>(
        &mut self,
        hook: F,
    ) {
        self.deadline_hook = Some(Arc::new(hook));
    }

    /// Request that the event loop exit after the current callback.
    pub const fn request_exit(&mut self) {
        self.exit_requested = true;
    }

    /// Return whether exit has been requested.
    #[must_use]
    pub const fn exit_requested(&self) -> bool {
        self.exit_requested
    }

    /// Return the live window for a handle, if it has been
    /// materialised yet.
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
    #[must_use]
    pub fn take_failed_windows(&mut self) -> Vec<(WinitWindowHandle, String)> {
        std::mem::take(&mut self.failed)
    }

    pub(crate) fn wake_hook(&self) -> Option<&WakeHook> {
        self.wake_hook.as_ref()
    }

    pub(crate) fn deadline_hook(&self) -> Option<&DeadlineHook> {
        self.deadline_hook.as_ref()
    }

    pub(crate) fn pending_wake(&self) -> &AtomicBool {
        &self.pending_wake
    }

    pub(crate) fn push_event(&mut self, event: ShellEvent) {
        self.events.push(event);
    }

    pub(crate) fn request_redraws(&self) {
        for (_, window) in &self.windows {
            window.request_redraw();
        }
    }

    /// Flag the next frame as one the change-gate must paint regardless of the
    /// dirty state — used for OS-driven redraws (first-show, un-occlude, resize)
    /// where the drawable needs pixels even though no signal changed.
    pub(crate) const fn set_force_paint(&mut self) {
        self.force_paint = true;
    }

    /// Take (and clear) the force-paint flag. Read once at the gate in the frame
    /// loop; returns `true` when this frame must paint irrespective of dirtiness.
    pub fn take_force_paint(&mut self) -> bool {
        std::mem::take(&mut self.force_paint)
    }

    const fn allocate_handle(&mut self) -> WinitWindowHandle {
        let id = self.next_handle;
        self.next_handle = self.next_handle.wrapping_add(1);
        WinitWindowHandle::from_raw(id)
    }

    pub(crate) fn materialise_pending(&mut self, active: &ActiveEventLoop) {
        if self.pending.is_empty() {
            return;
        }
        let pending = std::mem::take(&mut self.pending);
        for spec in pending {
            let size = match spec.size {
                PendingWindowSize::Physical(size) => {
                    WinitSize::Physical(WinitPhysicalSize::new(size.width, size.height))
                }
                PendingWindowSize::Logical(size) => WinitSize::Logical(WinitLogicalSize::new(
                    f64::from(size.width),
                    f64::from(size.height),
                )),
            };
            let attrs = WindowAttributes::default()
                .with_title(spec.title)
                .with_inner_size(size);
            match active.create_window(attrs) {
                Ok(window) => {
                    self.window_scales
                        .push((spec.handle, scale_from_f64(window.scale_factor())));
                    self.windows.push((spec.handle, Arc::new(window)));
                }
                // Recorded for `take_failed_windows`; callers surface
                // the error out-of-band since the shell cannot panic.
                Err(err) => self.failed.push((spec.handle, err.to_string())),
            }
        }
    }

    pub(crate) fn buffer_window_event(
        &mut self,
        window_id: WindowId,
        event: WinitWindowEvent,
    ) -> bool {
        let scale = scale_for_window_id(&self.windows, &self.window_scales, window_id);
        match event {
            WinitWindowEvent::Resized(size) => self.push_window_resized(size),
            WinitWindowEvent::Destroyed => self.push_window(SigilWindowEvent::Destroyed),
            WinitWindowEvent::Focused(focused) => self.push_focus(focused),
            WinitWindowEvent::KeyboardInput { event, .. } => self.push_key(event),
            WinitWindowEvent::ModifiersChanged(modifiers) => self.update_modifiers(modifiers),
            WinitWindowEvent::Ime(event) => self.push_ime(event),
            WinitWindowEvent::CursorMoved { position, .. } => {
                self.push_cursor_move(position, scale)
            }
            WinitWindowEvent::CursorEntered { .. } => self.push_pointer_phase(PointerPhase::Enter),
            WinitWindowEvent::CursorLeft { .. } => self.push_pointer_phase(PointerPhase::Leave),
            WinitWindowEvent::MouseWheel { delta, phase, .. } => {
                self.push_scroll(delta, phase, scale)
            }
            WinitWindowEvent::MouseInput { state, button, .. } => {
                self.push_mouse_input(state, button);
                true
            }
            WinitWindowEvent::Touch(touch) => {
                self.push_touch(touch.phase, touch.location.x, touch.location.y, scale);
                true
            }
            WinitWindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.push_scale_factor_changed(window_id, scale_factor)
            }
            WinitWindowEvent::Occluded(occluded) => {
                self.push_window(SigilWindowEvent::Occluded(occluded))
            }
            _ => false,
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
            size: PendingWindowSize::Physical(initial_size),
        });
        Ok(handle)
    }

    fn create_window_logical(
        &mut self,
        title: &str,
        initial: LogicalSize,
    ) -> Result<Self::Window, Self::Error> {
        let handle = self.allocate_handle();
        self.pending.push(PendingWindow {
            handle,
            title: title.to_owned(),
            size: PendingWindowSize::Logical(initial),
        });
        Ok(handle)
    }

    fn drain_events(&mut self) -> std::vec::IntoIter<ShellEvent> {
        std::mem::take(&mut self.events).into_iter()
    }

    fn run<F>(mut self, frame: F) -> Result<(), Self::Error>
    where
        F: FnMut(&mut Self) + 'static,
    {
        let event_loop = self.event_loop.take().ok_or(ShellError::AlreadyRun)?;
        let mut app = WinitApp {
            shell: self,
            frame,
            input_redraw_pending: false,
        };
        event_loop.run_app(&mut app)?;
        Ok(())
    }
}
