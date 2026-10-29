use std::cell::RefCell;

use crate::app::platform::{Bridge, Wake};
use crate::reactive::EventChannel;

pub use tray_icon::{Icon, TrayIcon, TrayIconBuilder, TrayIconEvent};

thread_local! {
    static PENDING: RefCell<Option<TrayIconBuilder>> = const { RefCell::new(None) };
    static INSTALLED: RefCell<Option<TrayIcon>> = const { RefCell::new(None) };
    static EVENTS: EventChannel<TrayIconEvent> = EventChannel::new();
    static BRIDGE: Bridge<TrayIconEvent> = Bridge::new();
}

pub fn set_tray(builder: TrayIconBuilder) {
    PENDING.with(|slot| *slot.borrow_mut() = Some(builder));
}

pub fn update<R>(edit: impl FnOnce(&TrayIcon) -> R) -> Option<R> {
    INSTALLED.with(|slot| slot.borrow().as_ref().map(edit))
}

#[must_use]
pub fn events() -> EventChannel<TrayIconEvent> {
    EVENTS.with(Clone::clone)
}

#[cfg(any(test, feature = "kittest"))]
pub fn simulate(event: TrayIconEvent) {
    events().emit(event);
}

pub(crate) fn has_pending() -> bool {
    PENDING.with(|slot| slot.borrow().is_some())
}

pub(crate) fn install(wake: Wake) {
    if has_pending() {
        install_tray(wake);
    }
}

pub(crate) fn pump() {
    BRIDGE.with(|bridge| EVENTS.with(|channel| bridge.forward(channel)));
}

fn install_tray(wake: Wake) {
    let Some(builder) = PENDING.with(|slot| slot.borrow_mut().take()) else {
        return;
    };
    match builder.build() {
        Ok(tray) => INSTALLED.with(|slot| *slot.borrow_mut() = Some(tray)),
        Err(err) => {
            tracing::warn!(target: "sigil::tray", error = %err, "tray icon build failed");
            return;
        }
    }

    let sink = BRIDGE.with(|bridge| bridge.sink(wake));
    TrayIconEvent::set_event_handler(Some(sink));
}

#[cfg(test)]
#[path = "tray_tests.rs"]
mod tests;
