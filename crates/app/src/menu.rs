//! Native menu bar.
//!
//! Build a menu with the re-exported [`muda`] types, register it with
//! [`set_menu_bar`]. Call before [`launch`](crate::launch) or any time
//! after; the next frame installs it. React to clicks on
//! [`events`].
//!
//! Native bar: macOS and Windows. Opt-in OS chrome. The bar you style
//! is `sigil::menu_bar`. Linux has no native bar on this host.
//!
//! ```rust,no_run
//! use sigil::menu::{self, Menu, MenuItem, PredefinedMenuItem, Submenu};
//!
//! let open = MenuItem::with_id("open", "Open…", true, None);
//! let file = Submenu::with_items("File", true, &[&open, &PredefinedMenuItem::quit(None)]).unwrap();
//! let bar = Menu::new();
//! bar.append(&file).unwrap();
//! menu::set_menu_bar(bar);
//!
//! // In your view, react to activations:
//! let _sub = menu::events().subscribe(|id| {
//!     if id == "open" { /* open a file */ }
//! });
//! ```
//!
//! Sigil installs a pending bar once the window exists, then again
//! whenever [`set_menu_bar`] queues a replacement. Clicks wake the
//! loop and emit on [`events`].

use std::cell::RefCell;

use crossbeam_channel::{Receiver, Sender, unbounded};
use sigil_reactive::EventChannel;

pub use muda::{
    AboutMetadata, CheckMenuItem, ContextMenu, IconMenuItem, Menu, MenuEvent, MenuId, MenuItem,
    PredefinedMenuItem, Submenu,
};

thread_local! {
    /// Menu registered by the app before launch, installed on first frame.
    static PENDING: RefCell<Option<Menu>> = const { RefCell::new(None) };
    /// The installed menu, kept alive for the process (drop removes the bar).
    static INSTALLED: RefCell<Option<Menu>> = const { RefCell::new(None) };
    /// Reactive stream apps subscribe to.
    static EVENTS: EventChannel<MenuId> = EventChannel::new();
    /// Send-safe bridge: muda's handler (any thread) -> main-thread frame.
    static BRIDGE: (Sender<MenuId>, Receiver<MenuId>) = unbounded();
}

/// Register the native menu bar. Call before [`launch`](crate::launch)
/// or any time after. The next frame installs or replaces the bar.
pub fn set_menu_bar(menu: Menu) {
    PENDING.with(|slot| *slot.borrow_mut() = Some(menu));
}

/// Reactive stream of menu-item activations. Subscribe and match the id.
#[must_use]
pub fn events() -> EventChannel<MenuId> {
    // A clone is a cheap handle to the same underlying channel, so emits from
    // `pump` reach subscribers made through the returned value.
    EVENTS.with(Clone::clone)
}

/// Simulate a menu-item activation, as if the user clicked the item with
/// this id. A subscriber on [`events`] fires synchronously — the test seam
/// for menu handling, no native menu bar or display required.
pub fn simulate(id: MenuId) {
    events().emit(id);
}

/// True while a menu is registered but not yet installed (first-frame gate).
pub(crate) fn has_pending() -> bool {
    PENDING.with(|slot| slot.borrow().is_some())
}

/// Install the pending menu. `hwnd` is the Win32 window handle on Windows,
/// ignored elsewhere. `wake` wakes the event loop when a menu event fires.
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

/// Drain bridged menu events into the reactive channel. Call each frame,
/// inside the effect phase, so subscriber effects run this frame.
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
        // SAFETY: `hwnd` is the live main-window handle from the shell.
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
