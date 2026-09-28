use std::cell::RefCell;

use crate::reactive::EventChannel;
use crossbeam_channel::{Receiver, Sender, unbounded};

pub use tray_icon::{Icon, TrayIcon, TrayIconBuilder, TrayIconEvent};

thread_local! {
    static PENDING: RefCell<Option<TrayIconBuilder>> = const { RefCell::new(None) };
    static INSTALLED: RefCell<Option<TrayIcon>> = const { RefCell::new(None) };
    static EVENTS: EventChannel<TrayIconEvent> = EventChannel::new();
    static BRIDGE: (Sender<TrayIconEvent>, Receiver<TrayIconEvent>) = unbounded();
}

pub fn set_tray(builder: TrayIconBuilder) {
    PENDING.with(|slot| *slot.borrow_mut() = Some(builder));
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

pub(crate) fn install(wake: impl Fn() + Send + Sync + 'static) {
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

    let tx = BRIDGE.with(|(tx, _)| tx.clone());
    TrayIconEvent::set_event_handler(Some(move |event: TrayIconEvent| {
        let _ = tx.send(event);
        wake();
    }));
}

pub(crate) fn pump() {
    BRIDGE.with(|(_, rx)| {
        EVENTS.with(|channel| {
            while let Ok(event) = rx.try_recv() {
                channel.emit(event);
            }
        });
    });
}

#[cfg(test)]
#[path = "tray_tests.rs"]
mod tests;
