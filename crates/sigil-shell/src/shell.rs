//! `WinitShell` — the winit-backed [`sigil_hal::Shell`] implementation.
//! See DR-002 for the lifecycle contract: windows are queued until the
//! event loop is alive, and the event loop is built eagerly so the
//! [`Shell::waker`] handle can be handed to `sigil-async` before
//! [`Shell::run`] is called.

#![allow(clippy::redundant_pub_crate)]

use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};

use sigil_hal::{
    ImeEvent, KeyEvent as SigilKeyEvent, KeyEventInit, KeyPhase, LogicalSize, ModifiersState,
    MouseButton, MouseButtons, PhysicalSize, Point, PointerEvent, PointerEventInit, PointerKind,
    PointerPhase, Scale, ScrollEvent, Shell, ShellEvent, WindowEvent as SigilWindowEvent,
};
use smol_str::SmolStr;
use winit::{
    dpi::{LogicalSize as WinitLogicalSize, PhysicalSize as WinitPhysicalSize, Size as WinitSize},
    event::{
        ElementState, Ime as WinitIme, KeyEvent as WinitKeyEvent, MouseButton as WinitMouseButton,
        MouseScrollDelta as WinitMouseScrollDelta, TouchPhase, WindowEvent as WinitWindowEvent,
    },
    event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy},
    window::{Window, WindowAttributes, WindowId},
};

use crate::{
    ShellError, WinitWindowHandle,
    app::WinitApp,
    dpi::{logical_point_from_f64, scale_for_window_id, scale_from_f64, set_scale_for_window_id},
    event_map::{
        map_modifiers, map_mouse_button, map_scroll_delta, map_scroll_phase,
        map_touch_pointer_phase, mouse_button_bit,
    },
    key_map::{map_logical_key, map_physical_key},
    waker::{WakeEvent, WinitWaker},
};

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
    exit_requested: bool,
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
            exit_requested: false,
        })
    }

    /// Register a closure invoked once per delivered `WakeEvent`
    /// inside `ApplicationHandler::user_event`.
    pub fn on_wake<F: Fn() + Send + Sync + 'static>(&mut self, hook: F) {
        self.wake_hook = Some(Arc::new(hook));
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

    fn push_window_resized(&mut self, size: WinitPhysicalSize<u32>) -> bool {
        self.push_window(SigilWindowEvent::Resized(PhysicalSize::new(
            size.width,
            size.height,
        )))
    }

    fn push_window(&mut self, event: SigilWindowEvent) -> bool {
        self.events.push(ShellEvent::Window(event));
        true
    }

    fn push_scale_factor_changed(&mut self, window_id: WindowId, scale_factor: f64) -> bool {
        let scale = scale_from_f64(scale_factor);
        set_scale_for_window_id(&self.windows, &mut self.window_scales, window_id, scale);
        self.push_window(SigilWindowEvent::ScaleFactorChanged(scale))
    }

    fn push_focus(&mut self, focused: bool) -> bool {
        self.push_window(SigilWindowEvent::Focused(focused));
        if !focused {
            self.push_pointer_phase(PointerPhase::Leave);
        }
        true
    }

    fn update_modifiers(&mut self, modifiers: winit::event::Modifiers) -> bool {
        self.modifiers = map_modifiers(modifiers.state());
        false
    }

    fn push_key(&mut self, event: WinitKeyEvent) -> bool {
        let phase = match event.state {
            ElementState::Pressed => KeyPhase::Down,
            ElementState::Released => KeyPhase::Up,
        };
        let text = event.text.as_ref().map(|text| SmolStr::new(text.as_str()));
        self.events
            .push(ShellEvent::Key(SigilKeyEvent::new(KeyEventInit {
                key: map_logical_key(event.logical_key),
                physical: map_physical_key(event.physical_key),
                modifiers: self.modifiers,
                repeat: event.repeat,
                phase,
                text,
            })));
        true
    }

    fn push_ime(&mut self, event: WinitIme) -> bool {
        let event = match event {
            WinitIme::Enabled => ImeEvent::Enabled,
            WinitIme::Preedit(text, range) => ImeEvent::Preedit(SmolStr::new(&text), range),
            WinitIme::Commit(text) => ImeEvent::Commit(SmolStr::new(&text)),
            WinitIme::Disabled => ImeEvent::Disabled,
        };
        self.events.push(ShellEvent::Ime(event));
        true
    }

    fn push_cursor_move(
        &mut self,
        position: winit::dpi::PhysicalPosition<f64>,
        scale: Scale,
    ) -> bool {
        let next = logical_point_from_f64(position.x, position.y, scale);
        let delta = Point::new(
            next.x - self.pointer_position.x,
            next.y - self.pointer_position.y,
        );
        self.pointer_position = next;
        self.push_pointer(PointerPhase::Move, delta, None, PointerKind::Mouse);
        true
    }

    fn push_pointer_phase(&mut self, phase: PointerPhase) -> bool {
        self.push_pointer(phase, Point::new(0.0, 0.0), None, PointerKind::Mouse);
        true
    }

    fn push_mouse_input(&mut self, state: ElementState, button: WinitMouseButton) {
        let button = map_mouse_button(button);
        if let Some(bit) = mouse_button_bit(button) {
            match state {
                ElementState::Pressed => self.pointer_buttons.insert(bit),
                ElementState::Released => self.pointer_buttons.remove(bit),
            }
        }
        let phase = match state {
            ElementState::Pressed => PointerPhase::Down,
            ElementState::Released => PointerPhase::Up,
        };
        self.push_pointer(
            phase,
            Point::new(0.0, 0.0),
            Some(button),
            PointerKind::Mouse,
        );
    }

    fn push_touch(&mut self, phase: TouchPhase, x: f64, y: f64, scale: Scale) {
        let next = logical_point_from_f64(x, y, scale);
        let delta = Point::new(
            next.x - self.pointer_position.x,
            next.y - self.pointer_position.y,
        );
        self.pointer_position = next;
        self.push_pointer(
            map_touch_pointer_phase(phase),
            delta,
            None,
            PointerKind::Touch,
        );
    }

    fn push_pointer(
        &mut self,
        phase: PointerPhase,
        delta: Point,
        button: Option<MouseButton>,
        kind: PointerKind,
    ) {
        self.events
            .push(ShellEvent::Pointer(PointerEvent::new(PointerEventInit {
                position: self.pointer_position,
                delta,
                kind,
                button,
                buttons: self.pointer_buttons,
                modifiers: self.modifiers,
                phase,
            })));
    }

    fn push_scroll(
        &mut self,
        delta: WinitMouseScrollDelta,
        phase: TouchPhase,
        scale: Scale,
    ) -> bool {
        self.events.push(ShellEvent::Scroll(ScrollEvent {
            position: self.pointer_position,
            delta: map_scroll_delta(delta, scale),
            modifiers: self.modifiers,
            phase: map_scroll_phase(phase),
        }));
        true
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
        let mut app = WinitApp { shell: self, frame };
        event_loop.run_app(&mut app)?;
        Ok(())
    }
}
