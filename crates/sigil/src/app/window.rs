use std::cell::{Cell, RefCell};

use crate::app::WindowCommand;
use crate::app::config::AppConfig;
use crate::app::embed::Host;
use crate::kernel::IntoNode;
use crate::reactive::{Signal, signal};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WindowId(u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum CloseAction {
    #[default]
    Close,
    Hide,
    Keep,
}

#[derive(Clone, Copy)]
pub(crate) struct WindowSignals {
    pub(crate) focused: Signal<bool>,
    pub(crate) occluded: Signal<bool>,
}

pub(crate) type Mount = Box<dyn FnOnce(&mut Host)>;

pub(crate) struct OpenRequest {
    pub(crate) id: WindowId,
    pub(crate) config: AppConfig,
    pub(crate) mount: Mount,
}

thread_local! {
    static NEXT_ID: Cell<u64> = const { Cell::new(0) };
    static CURRENT: Cell<Option<WindowId>> = const { Cell::new(None) };
    static OPENS: RefCell<Vec<OpenRequest>> = const { RefCell::new(Vec::new()) };
    static COMMANDS: RefCell<Vec<(Option<WindowId>, WindowCommand)>> = const { RefCell::new(Vec::new()) };
    static SIGNALS: RefCell<Vec<(WindowId, WindowSignals)>> = const { RefCell::new(Vec::new()) };
}

impl WindowId {
    pub fn close(self) {
        self.command(WindowCommand::Close);
    }

    pub fn command(self, command: WindowCommand) {
        queue_command(Some(self), command);
    }

    #[must_use]
    pub fn focused(self) -> Option<Signal<bool>> {
        signals_for(self).map(|s| s.focused)
    }

    #[must_use]
    pub fn occluded(self) -> Option<Signal<bool>> {
        signals_for(self).map(|s| s.occluded)
    }
}

pub fn open_window<C, F, N>(config: C, view: F) -> WindowId
where
    C: Into<AppConfig>,
    F: Fn() -> N + 'static,
    N: IntoNode,
{
    let id = allocate_window_id();
    let request = OpenRequest {
        id,
        config: config.into(),
        mount: Box::new(move |host: &mut Host| host.mount_app(view)),
    };
    OPENS.with(|cell| cell.borrow_mut().push(request));
    id
}

#[must_use]
pub fn current_window() -> Option<WindowId> {
    CURRENT.with(Cell::get)
}

#[must_use]
pub fn window_focused() -> Signal<bool> {
    current_signals().focused
}

#[must_use]
pub fn window_occluded() -> Signal<bool> {
    current_signals().occluded
}

fn current_signals() -> WindowSignals {
    let current = current_window().and_then(signals_for);
    current
        .or_else(|| SIGNALS.with(|cell| cell.borrow().first().map(|(_, s)| *s)))
        .unwrap_or_else(fresh_signals)
}

fn fresh_signals() -> WindowSignals {
    WindowSignals {
        focused: signal(true),
        occluded: signal(false),
    }
}

fn signals_for(id: WindowId) -> Option<WindowSignals> {
    SIGNALS.with(|cell| {
        cell.borrow()
            .iter()
            .find_map(|(stored, s)| (*stored == id).then_some(*s))
    })
}

pub(crate) fn allocate_window_id() -> WindowId {
    NEXT_ID.with(|next| {
        let id = next.get();
        next.set(id.wrapping_add(1));
        WindowId(id)
    })
}

pub(crate) fn register_window(id: WindowId) -> WindowSignals {
    let signals = fresh_signals();
    SIGNALS.with(|cell| cell.borrow_mut().push((id, signals)));
    signals
}

pub(crate) fn unregister_window(id: WindowId) {
    SIGNALS.with(|cell| cell.borrow_mut().retain(|(stored, _)| *stored != id));
}

pub(crate) fn set_current_window(id: Option<WindowId>) -> Option<WindowId> {
    CURRENT.with(|cell| cell.replace(id))
}

pub(crate) fn take_open_requests() -> Vec<OpenRequest> {
    OPENS.with(|cell| core::mem::take(&mut *cell.borrow_mut()))
}

pub(crate) fn queue_command(target: Option<WindowId>, command: WindowCommand) {
    COMMANDS.with(|cell| cell.borrow_mut().push((target, command)));
}

pub(crate) fn take_commands() -> Vec<(Option<WindowId>, WindowCommand)> {
    COMMANDS.with(|cell| core::mem::take(&mut *cell.borrow_mut()))
}

pub(crate) fn reset() {
    CURRENT.with(|cell| cell.set(None));
    OPENS.with(|cell| cell.borrow_mut().clear());
    SIGNALS.with(|cell| cell.borrow_mut().clear());
}
