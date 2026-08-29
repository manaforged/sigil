use std::cell::RefCell;

use crossbeam_channel::{Receiver, Sender, unbounded};
use sigil_reactive::EventChannel;

pub use muda::{
    AboutMetadata, CheckMenuItem, ContextMenu, IconMenuItem, Menu, MenuEvent, MenuId, MenuItem,
    PredefinedMenuItem, Submenu,
};

thread_local! {
    static PENDING: RefCell<Option<Menu>> = const { RefCell::new(None) };
    static INSTALLED: RefCell<Option<Menu>> = const { RefCell::new(None) };
    static EVENTS: EventChannel<MenuId> = EventChannel::new();
    static BRIDGE: (Sender<MenuId>, Receiver<MenuId>) = unbounded();
}

pub fn set_menu_bar(menu: Menu) {
    PENDING.with(|slot| *slot.borrow_mut() = Some(menu));
}

#[must_use]
pub fn events() -> EventChannel<MenuId> {
    EVENTS.with(Clone::clone)
}

pub fn simulate(id: MenuId) {
    events().emit(id);
}

pub(crate) fn has_pending() -> bool {
    PENDING.with(|slot| slot.borrow().is_some())
}

pub(crate) fn install(hwnd: Option<isize>, wake: impl Fn() + Send + Sync + 'static) {
    let Some(menu) = PENDING.with(|slot| slot.borrow_mut().take()) else {
        return;
    };
    install_native(&menu, hwnd);
    INSTALLED.with(|slot| *slot.borrow_mut() = Some(menu));

    let tx = BRIDGE.with(|(tx, _)| tx.clone());
    MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
        let _ = tx.send(event.id);
        wake();
    }));
}

pub(crate) fn pump() {
    BRIDGE.with(|(_, rx)| {
        EVENTS.with(|channel| {
            while let Ok(id) = rx.try_recv() {
                channel.emit(id);
            }
        });
    });
}

#[cfg(target_os = "macos")]
fn install_native(menu: &Menu, _hwnd: Option<isize>) {
    menu.init_for_nsapp();
}

#[cfg(target_os = "windows")]
fn install_native(menu: &Menu, hwnd: Option<isize>) {
    if let Some(hwnd) = hwnd {
        unsafe {
            if let Err(err) = menu.init_for_hwnd(hwnd) {
                tracing::warn!(
                    target: "sigil::menu",
                    error = %err,
                    "native menu bar did not attach to the window"
                );
            }
        }
    }
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn install_native(_menu: &Menu, _hwnd: Option<isize>) {
    tracing::warn!(
        target: "sigil::menu",
        "native menu bar is macOS and Windows; use sigil::widgets::menu on this host"
    );
}

#[cfg(test)]
#[path = "menu_tests.rs"]
mod tests;
