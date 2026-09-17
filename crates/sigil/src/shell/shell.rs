#![allow(clippy::redundant_pub_crate)]

use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::time::Instant;

use crate::hal::{
    LogicalSize, ModifiersState, MouseButtons, PhysicalSize, Point, Rect, Scale, Shell, ShellEvent,
    WindowCommand, WindowOpts,
};
use winit::{
    dpi::{LogicalPosition as WinitLogicalPosition, LogicalSize as WinitLogicalSize},
    event_loop::{EventLoop, EventLoopProxy},
    window::Window,
};

use crate::shell::{
    ShellError, WinitWindowHandle,
    app::WinitApp,
    waker::{WakeEvent, WinitWaker},
};

mod a11y;
mod attrs;
mod commands;
mod events;
mod overlay;

pub(crate) struct PendingWindow {
    handle: WinitWindowHandle,
    title: String,
    size: PendingWindowSize,
    opts: WindowOpts,
}

pub(crate) enum PendingWindowSize {
    Physical(PhysicalSize),
    Logical(LogicalSize),
}

pub(crate) type WakeHook = Arc<dyn Fn() + Send + Sync + 'static>;

pub(crate) type DeadlineHook = Arc<dyn Fn() -> Option<Instant> + Send + Sync + 'static>;

type A11yAdapterEntry = (
    WinitWindowHandle,
    accesskit_winit::Adapter,
    Arc<a11y::Activation>,
);

pub struct WinitShell {
    event_loop: Option<EventLoop<WakeEvent>>,
    proxy: Arc<Mutex<EventLoopProxy<WakeEvent>>>,
    pending_wake: Arc<AtomicBool>,
    alive: Arc<AtomicBool>,
    pending: Vec<PendingWindow>,
    windows: Vec<(WinitWindowHandle, Arc<Window>)>,
    window_scales: Vec<(WinitWindowHandle, Scale)>,
    window_zooms: Vec<(WinitWindowHandle, f32)>,
    failed: Vec<(WinitWindowHandle, String)>,
    events: Vec<ShellEvent>,
    pointer_position: Point,
    pointer_buttons: MouseButtons,
    modifiers: ModifiersState,
    next_handle: u64,
    wake_hook: Option<WakeHook>,
    deadline_hook: Option<DeadlineHook>,
    exit_requested: bool,
    force_paint: bool,
    a11y_adapters: Vec<A11yAdapterEntry>,
    a11y_action_tx: crossbeam_channel::Sender<accesskit::ActionRequest>,
    a11y_action_rx: crossbeam_channel::Receiver<accesskit::ActionRequest>,
}

impl Drop for WinitShell {
    fn drop(&mut self) {
        self.alive.store(false, Ordering::Release);
    }
}

impl WinitShell {
    pub fn new() -> Result<Self, ShellError> {
        let event_loop = EventLoop::<WakeEvent>::with_user_event().build()?;
        Ok(Self::from_event_loop(event_loop))
    }

    pub fn new_accessory() -> Result<Self, ShellError> {
        let mut builder = EventLoop::<WakeEvent>::with_user_event();
        #[cfg(target_os = "macos")]
        {
            use winit::platform::macos::{ActivationPolicy, EventLoopBuilderExtMacOS};
            builder.with_activation_policy(ActivationPolicy::Accessory);
        }
        let event_loop = builder.build()?;
        Ok(Self::from_event_loop(event_loop))
    }

    fn from_event_loop(event_loop: EventLoop<WakeEvent>) -> Self {
        let proxy = Arc::new(Mutex::new(event_loop.create_proxy()));
        let (a11y_action_tx, a11y_action_rx) = crossbeam_channel::unbounded();
        Self {
            event_loop: Some(event_loop),
            proxy,
            pending_wake: Arc::new(AtomicBool::new(false)),
            alive: Arc::new(AtomicBool::new(true)),
            pending: Vec::new(),
            windows: Vec::new(),
            window_scales: Vec::new(),
            window_zooms: Vec::new(),
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
            a11y_adapters: Vec::new(),
            a11y_action_tx,
            a11y_action_rx,
        }
    }

    pub fn on_wake<F: Fn() + Send + Sync + 'static>(&mut self, hook: F) {
        self.wake_hook = Some(Arc::new(hook));
    }

    pub fn on_idle_deadline<F: Fn() -> Option<Instant> + Send + Sync + 'static>(
        &mut self,
        hook: F,
    ) {
        self.deadline_hook = Some(Arc::new(hook));
    }

    pub const fn request_exit(&mut self) {
        self.exit_requested = true;
    }

    #[must_use]
    pub const fn exit_requested(&self) -> bool {
        self.exit_requested
    }

    #[must_use]
    pub fn window_for(&self, handle: WinitWindowHandle) -> Option<&Arc<Window>> {
        self.windows
            .iter()
            .find_map(|(h, w)| (*h == handle).then_some(w))
    }

    #[must_use]
    pub fn windows(&self) -> &[(WinitWindowHandle, Arc<Window>)] {
        &self.windows
    }

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

    pub(crate) const fn set_force_paint(&mut self) {
        self.force_paint = true;
    }

    pub fn take_force_paint(&mut self) -> bool {
        std::mem::take(&mut self.force_paint)
    }

    const fn allocate_handle(&mut self) -> WinitWindowHandle {
        let id = self.next_handle;
        self.next_handle = self.next_handle.wrapping_add(1);
        WinitWindowHandle::from_raw(id)
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
            opts: WindowOpts::default(),
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
            opts: WindowOpts::default(),
        });
        Ok(handle)
    }

    fn create_window_with(
        &mut self,
        title: &str,
        initial: LogicalSize,
        opts: WindowOpts,
    ) -> Result<Self::Window, Self::Error> {
        let handle = self.allocate_handle();
        self.pending.push(PendingWindow {
            handle,
            title: title.to_owned(),
            size: PendingWindowSize::Logical(initial),
            opts,
        });
        Ok(handle)
    }

    fn drain_events(&mut self) -> std::vec::IntoIter<ShellEvent> {
        std::mem::take(&mut self.events).into_iter()
    }

    fn set_ime_allowed(&mut self, window: &Self::Window, allowed: bool) -> Result<(), Self::Error> {
        if let Some(window) = self.window_for(*window) {
            window.set_ime_allowed(allowed);
        }
        Ok(())
    }

    fn set_ime_cursor_area(
        &mut self,
        window: &Self::Window,
        rect: Rect,
    ) -> Result<(), Self::Error> {
        if let Some(window) = self.window_for(*window) {
            window.set_ime_cursor_area(
                WinitLogicalPosition::new(f64::from(rect.origin.x), f64::from(rect.origin.y)),
                WinitLogicalSize::new(f64::from(rect.width), f64::from(rect.height)),
            );
        }
        Ok(())
    }

    fn window_command(
        &mut self,
        window: &Self::Window,
        command: WindowCommand,
    ) -> Result<(), Self::Error> {
        self.run_window_command(*window, command);
        Ok(())
    }

    fn run<F>(mut self, frame: F) -> Result<(), Self::Error>
    where
        F: FnMut(&mut Self) + 'static,
    {
        let event_loop = self.event_loop.take().ok_or(ShellError::AlreadyRun)?;
        let mut app = WinitApp {
            shell: self,
            frame,
            app_redraw_pending: false,
        };
        event_loop.run_app(&mut app)?;
        Ok(())
    }
}
