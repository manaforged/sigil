#![allow(clippy::redundant_pub_crate)]

use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use web_time::Instant;

use crate::app::WindowOpts;
use crate::events::{ModifiersState, MouseButtons, ShellEvent};
use crate::geometry::{LogicalSize, PhysicalSize, Point, Scale};
use winit::{
    event_loop::{EventLoop, EventLoopProxy},
    window::Window,
};

use crate::shell::{WinitWindowHandle, waker::WakeEvent};

mod a11y;
mod attrs;
mod commands;
mod events;
mod shell_impl;
#[cfg(target_arch = "wasm32")]
mod web;
#[cfg(target_arch = "wasm32")]
mod web_ime;

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

pub(crate) type DeadlineHook = Arc<dyn Fn() -> Option<Instant> + Send + Sync + 'static>;

pub(crate) type TaggedEvent = (WinitWindowHandle, ShellEvent);

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
    events: Vec<TaggedEvent>,
    event_window: WinitWindowHandle,
    redraw_target: Option<WinitWindowHandle>,
    pointer_position: Point,
    pointer_buttons: MouseButtons,
    modifiers: ModifiersState,
    next_handle: u64,
    deadline_hook: Option<DeadlineHook>,
    exit_requested: bool,
    force_paint: bool,
    a11y_adapters: Vec<a11y::AdapterEntry>,
    #[cfg(target_arch = "wasm32")]
    canvas: Option<String>,
    #[cfg(target_arch = "wasm32")]
    web_ime: Option<web_ime::WebIme>,
}

impl Drop for WinitShell {
    fn drop(&mut self) {
        self.alive.store(false, Ordering::Release);
    }
}

impl WinitShell {
    pub fn new() -> Result<Self, crate::shell::ShellError> {
        let event_loop = EventLoop::<WakeEvent>::with_user_event().build()?;
        Ok(Self::from_event_loop(event_loop))
    }

    pub fn new_accessory() -> Result<Self, crate::shell::ShellError> {
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
            event_window: WinitWindowHandle::from_raw(0),
            redraw_target: None,
            pointer_position: Point::new(0.0, 0.0),
            pointer_buttons: MouseButtons::empty(),
            modifiers: ModifiersState::empty(),
            next_handle: 0,
            deadline_hook: None,
            exit_requested: false,
            force_paint: false,
            a11y_adapters: Vec::new(),
            #[cfg(target_arch = "wasm32")]
            canvas: None,
            #[cfg(target_arch = "wasm32")]
            web_ime: None,
        }
    }

    pub fn on_idle_deadline<F: Fn() -> Option<Instant> + Send + Sync + 'static>(
        &mut self,
        hook: F,
    ) {
        self.deadline_hook = Some(Arc::new(hook));
    }

    pub fn on_exit<F: FnOnce() + 'static>(&mut self, hook: F) {
        super::exit_hook::set(Box::new(hook));
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

    fn window_zoom(&self, handle: WinitWindowHandle) -> f32 {
        self.window_zooms
            .iter()
            .find_map(|(stored, zoom)| (*stored == handle).then_some(*zoom))
            .unwrap_or(1.0)
    }

    #[must_use]
    pub fn windows(&self) -> &[(WinitWindowHandle, Arc<Window>)] {
        &self.windows
    }

    #[must_use]
    pub fn take_failed_windows(&mut self) -> Vec<(WinitWindowHandle, String)> {
        std::mem::take(&mut self.failed)
    }

    pub(crate) const fn has_failed_windows(&self) -> bool {
        !self.failed.is_empty()
    }

    pub(crate) fn deadline_hook(&self) -> Option<&DeadlineHook> {
        self.deadline_hook.as_ref()
    }

    pub(crate) fn pending_wake(&self) -> &AtomicBool {
        &self.pending_wake
    }

    pub(crate) fn push_event(&mut self, handle: WinitWindowHandle, event: ShellEvent) {
        self.events.push((handle, event));
    }

    pub(crate) fn drain_tagged_events(&mut self) -> std::vec::IntoIter<TaggedEvent> {
        #[cfg(target_arch = "wasm32")]
        if let Some(ime) = &self.web_ime {
            let mut untagged = Vec::new();
            ime.drain_into(&mut untagged);
            let owner = self.windows.first().map_or(self.event_window, |(h, _)| *h);
            self.events
                .extend(untagged.into_iter().map(|event| (owner, event)));
        }
        std::mem::take(&mut self.events).into_iter()
    }

    #[must_use]
    pub const fn redraw_target(&self) -> Option<WinitWindowHandle> {
        self.redraw_target
    }

    pub(crate) const fn set_redraw_target(&mut self, target: Option<WinitWindowHandle>) {
        self.redraw_target = target;
    }

    pub(crate) fn handle_for(&self, id: winit::window::WindowId) -> Option<WinitWindowHandle> {
        self.windows
            .iter()
            .find_map(|(h, w)| (w.id() == id).then_some(*h))
    }

    pub(crate) fn any_visible(&self) -> bool {
        self.windows
            .iter()
            .any(|(_, window)| window.is_visible().unwrap_or(true))
    }

    pub fn remove_window(&mut self, handle: WinitWindowHandle) {
        self.windows.retain(|(h, _)| *h != handle);
        self.window_scales.retain(|(h, _)| *h != handle);
        self.window_zooms.retain(|(h, _)| *h != handle);
        self.a11y_adapters.retain(|entry| entry.handle != handle);
        self.events.retain(|(h, _)| *h != handle);
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
