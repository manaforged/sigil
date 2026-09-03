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

pub(crate) fn install(
    hwnd: Option<isize>,
    wake: impl Fn() + Send + Sync + 'static,
) -> Result<(), String> {
    let Some(menu) = PENDING.with(|slot| slot.borrow_mut().take()) else {
        return Ok(());
    };
    install_native(&menu, hwnd)?;
    INSTALLED.with(|slot| *slot.borrow_mut() = Some(menu));

    let tx = BRIDGE.with(|(tx, _)| tx.clone());
    MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
        let _ = tx.send(event.id);
        wake();
    }));
    Ok(())
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
#[expect(
    clippy::unnecessary_wraps,
    reason = "all native menu adapters share one fallible interface"
)]
fn install_native(menu: &Menu, _hwnd: Option<isize>) -> Result<(), String> {
    menu.init_for_nsapp();
    Ok(())
}

#[cfg(target_os = "windows")]
#[expect(
    unsafe_code,
    reason = "muda requires a raw live HWND to install the native menu"
)]
fn install_native(menu: &Menu, hwnd: Option<isize>) -> Result<(), String> {
    let hwnd = hwnd.ok_or_else(|| "main window has no Win32 handle".to_string())?;
    // SAFETY: `hwnd` comes from the live winit window while the shell still
    // owns that window. `INSTALLED` keeps `menu` alive after this call.
    unsafe { menu.init_for_hwnd(hwnd) }.map_err(|error| error.to_string())
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn install_native(_menu: &Menu, _hwnd: Option<isize>) -> Result<(), String> {
    Err("native menu bars are unsupported on this platform".to_string())
}

#[cfg(test)]
#[path = "menu_tests.rs"]
mod tests;
