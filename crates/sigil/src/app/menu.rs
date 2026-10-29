use std::cell::RefCell;

use crate::app::platform::{Bridge, Wake};
use crate::events::{Key, LogicalKey, ModifiersState, Shortcut};
use crate::reactive::EventChannel;
use muda::accelerator::{Key as NativeKey, Modifiers as NativeModifiers};

pub use muda::accelerator::KeyAccelerator;
pub use muda::{
    AboutMetadata, CheckMenuItem, ContextMenu, IconMenuItem, Menu, MenuEvent, MenuId,
    MenuItem as NativeMenuItem, PredefinedMenuItem, Submenu,
};

thread_local! {
    static PENDING: RefCell<Option<Menu>> = const { RefCell::new(None) };
    static INSTALLED: RefCell<Option<Menu>> = const { RefCell::new(None) };
    static EVENTS: EventChannel<MenuId> = EventChannel::new();
    static BRIDGE: Bridge<MenuId> = Bridge::new();
}

pub fn set_menu_bar(menu: Menu) {
    PENDING.with(|slot| *slot.borrow_mut() = Some(menu));
}

#[must_use]
pub fn events() -> EventChannel<MenuId> {
    EVENTS.with(Clone::clone)
}

#[cfg(any(test, feature = "kittest"))]
pub fn simulate(id: MenuId) {
    events().emit(id);
}

pub(crate) fn has_pending() -> bool {
    PENDING.with(|slot| slot.borrow().is_some())
}

pub(crate) fn install(hwnd: Option<isize>, wake: Wake) {
    if has_pending()
        && let Err(error) = install_menu(hwnd, wake)
    {
        tracing::error!("sigil-app: cannot install native menu: {error}");
    }
}

pub(crate) fn pump() {
    BRIDGE.with(|bridge| EVENTS.with(|channel| bridge.forward(channel)));
}

fn install_menu(hwnd: Option<isize>, wake: Wake) -> Result<(), String> {
    let Some(menu) = PENDING.with(|slot| slot.borrow_mut().take()) else {
        return Ok(());
    };
    install_native(&menu, hwnd)?;
    INSTALLED.with(|slot| *slot.borrow_mut() = Some(menu));

    let sink = BRIDGE.with(|bridge| bridge.sink(wake));
    MenuEvent::set_event_handler(Some(move |event: MenuEvent| sink(event.id)));
    Ok(())
}

#[must_use]
pub fn accelerator(shortcut: &Shortcut) -> Option<KeyAccelerator> {
    let key = match &shortcut.key {
        LogicalKey::Named(key) => native_named_key(*key)?,
        LogicalKey::Character(text) => NativeKey::Character(text.to_string()),
    };
    Some(KeyAccelerator::new(
        Some(native_modifiers(shortcut.modifiers)),
        key,
    ))
}

fn native_modifiers(modifiers: ModifiersState) -> NativeModifiers {
    let pairs = [
        (ModifiersState::SHIFT, NativeModifiers::SHIFT),
        (ModifiersState::CONTROL, NativeModifiers::CONTROL),
        (ModifiersState::ALT, NativeModifiers::ALT),
        (ModifiersState::META, NativeModifiers::SUPER),
    ];
    pairs
        .into_iter()
        .filter(|(ours, _)| modifiers.contains(*ours))
        .fold(NativeModifiers::empty(), |acc, (_, native)| acc | native)
}

const fn native_named_key(key: Key) -> Option<NativeKey> {
    Some(match key {
        Key::Enter => NativeKey::Enter,
        Key::Tab => NativeKey::Tab,
        Key::Escape => NativeKey::Escape,
        Key::Backspace => NativeKey::Backspace,
        Key::Delete => NativeKey::Delete,
        Key::Insert => NativeKey::Insert,
        Key::Home => NativeKey::Home,
        Key::End => NativeKey::End,
        Key::PageUp => NativeKey::PageUp,
        Key::PageDown => NativeKey::PageDown,
        Key::ArrowUp => NativeKey::ArrowUp,
        Key::ArrowDown => NativeKey::ArrowDown,
        Key::ArrowLeft => NativeKey::ArrowLeft,
        Key::ArrowRight => NativeKey::ArrowRight,
        Key::F1 => NativeKey::F1,
        Key::F2 => NativeKey::F2,
        Key::F3 => NativeKey::F3,
        Key::F4 => NativeKey::F4,
        Key::F5 => NativeKey::F5,
        Key::F6 => NativeKey::F6,
        Key::F7 => NativeKey::F7,
        Key::F8 => NativeKey::F8,
        Key::F9 => NativeKey::F9,
        Key::F10 => NativeKey::F10,
        Key::F11 => NativeKey::F11,
        Key::F12 => NativeKey::F12,
        _ => return None,
    })
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
    // SAFETY: `hwnd` comes from the live winit window the shell still owns; `INSTALLED` keeps `menu` alive after this call.
    unsafe { menu.init_for_hwnd(hwnd) }.map_err(|error| error.to_string())
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn install_native(_menu: &Menu, _hwnd: Option<isize>) -> Result<(), String> {
    Err("native menu bars are unsupported on this platform".to_string())
}

#[cfg(test)]
#[path = "menu_tests.rs"]
mod tests;
