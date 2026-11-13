use std::cell::RefCell;

use crate::app::WindowCommand;
use crate::app::config::AppConfig;
use crate::app::embed::Host;
use crate::kernel::IntoNode;
use crate::reactive::{Signal, signal};

/// Open another window with its own tree on the same event loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Window(u64);

/// What happens when the user closes a window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum CloseAction {
    /// Closes the window.
    #[default]
    Close,
    /// Hides the window and keeps it alive.
    Hide,
    /// Leaves the window open.
    Keep,
}

#[derive(Clone, Copy)]
pub(crate) struct WindowSignals {
    pub(crate) focused: Signal<bool>,
    pub(crate) occluded: Signal<bool>,
}

pub(crate) type Mount = Box<dyn FnOnce(&mut Host)>;

pub(crate) struct OpenRequest {
    pub(crate) id: Window,
    pub(crate) config: AppConfig,
    pub(crate) mount: Mount,
}

struct WindowRegistry {
    next_id: u64,
    current: Option<Window>,
    opens: Vec<OpenRequest>,
    commands: Vec<(Option<Window>, WindowCommand)>,
    signals: Vec<(Window, WindowSignals)>,
}

impl WindowRegistry {
    const fn new() -> Self {
        Self {
            next_id: 0,
            current: None,
            opens: Vec::new(),
            commands: Vec::new(),
            signals: Vec::new(),
        }
    }
}

thread_local! {
    static REGISTRY: RefCell<WindowRegistry> = const { RefCell::new(WindowRegistry::new()) };
}

impl Window {
    /// Opens another window with its own tree.
    pub fn open<C, F, N>(config: C, view: F) -> Self
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
        REGISTRY.with(|cell| cell.borrow_mut().opens.push(request));
        id
    }

    /// The window whose tree is being mounted or updated; `None` outside a window.
    #[must_use]
    pub fn current() -> Option<Self> {
        current_window()
    }

    /// Closes this window.
    pub fn close(self) {
        self.command(WindowCommand::Close);
    }

    /// Sends a command to this window.
    pub fn command(self, command: WindowCommand) {
        queue_command(Some(self), command);
    }

    /// A signal that is true while the window has focus; `None` when the window is not open.
    #[must_use]
    pub fn focused(self) -> Option<Signal<bool>> {
        signals_for(self).map(|s| s.focused)
    }

    /// A signal that is true while the window is occluded; `None` when the window is not open.
    #[must_use]
    pub fn occluded(self) -> Option<Signal<bool>> {
        signals_for(self).map(|s| s.occluded)
    }
}

#[must_use]
pub(crate) fn current_window() -> Option<Window> {
    REGISTRY.with(|cell| cell.borrow().current)
}

fn fresh_signals() -> WindowSignals {
    WindowSignals {
        focused: signal(true),
        occluded: signal(false),
    }
}

fn signals_for(id: Window) -> Option<WindowSignals> {
    REGISTRY.with(|cell| {
        cell.borrow()
            .signals
            .iter()
            .find_map(|(stored, s)| (*stored == id).then_some(*s))
    })
}

pub(crate) fn allocate_window_id() -> Window {
    REGISTRY.with(|cell| {
        let mut reg = cell.borrow_mut();
        let id = reg.next_id;
        reg.next_id = id.wrapping_add(1);
        Window(id)
    })
}

pub(crate) fn register_window(id: Window) -> WindowSignals {
    let signals = fresh_signals();
    REGISTRY.with(|cell| cell.borrow_mut().signals.push((id, signals)));
    signals
}

pub(crate) fn unregister_window(id: Window) {
    REGISTRY.with(|cell| {
        cell.borrow_mut()
            .signals
            .retain(|(stored, _)| *stored != id);
    });
}

pub(crate) fn set_current_window(id: Option<Window>) -> Option<Window> {
    REGISTRY.with(|cell| core::mem::replace(&mut cell.borrow_mut().current, id))
}

pub(crate) fn take_open_requests() -> Vec<OpenRequest> {
    REGISTRY.with(|cell| core::mem::take(&mut cell.borrow_mut().opens))
}

pub(crate) fn queue_command(target: Option<Window>, command: WindowCommand) {
    REGISTRY.with(|cell| cell.borrow_mut().commands.push((target, command)));
}

pub(crate) fn take_commands() -> Vec<(Option<Window>, WindowCommand)> {
    REGISTRY.with(|cell| core::mem::take(&mut cell.borrow_mut().commands))
}

pub(crate) fn reset() {
    REGISTRY.with(|cell| {
        let mut reg = cell.borrow_mut();
        reg.current = None;
        reg.opens.clear();
        reg.commands.clear();
        reg.signals.clear();
    });
}

#[cfg(test)]
#[path = "window_tests.rs"]
mod tests;
