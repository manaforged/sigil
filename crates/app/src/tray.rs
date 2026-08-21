//! System tray icon.
//!
//! Build a tray icon with the re-exported [`tray_icon`] types, register it
//! with [`set_tray`](crate::tray::set_tray) BEFORE `launch`; Sigil creates it on the first frame and
//! forwards tray-*icon* clicks to the reactive [`events`](crate::tray::events) channel. A tray
//! context menu is a [`muda`](crate::menu) menu, so its item clicks arrive
//! through [`menu::events`](crate::menu::events), not here.
//!
//! ```rust,no_run
//! use sigil::tray::{self, Icon, TrayIconBuilder};
//!
//! let icon = Icon::from_rgba(vec![0; 16 * 16 * 4], 16, 16).unwrap();
//! tray::set_tray(TrayIconBuilder::new().with_tooltip("My app").with_icon(icon));
//!
//! let _sub = tray::events().subscribe(|event| {
//!     // left/right/double click on the tray icon
//!     let _ = event;
//! });
//! ```
//!
//! Native on macOS + Windows; Linux needs a StatusNotifier host.

use std::cell::RefCell;

use crossbeam_channel::{Receiver, Sender, unbounded};
use sigil_reactive::EventChannel;

pub use tray_icon::{Icon, TrayIcon, TrayIconBuilder, TrayIconEvent};

thread_local! {
    /// Builder registered before launch, built on the first frame.
    static PENDING: RefCell<Option<TrayIconBuilder>> = const { RefCell::new(None) };
    /// The live tray icon, kept alive for the process (drop removes it).
    static INSTALLED: RefCell<Option<TrayIcon>> = const { RefCell::new(None) };
    /// Reactive stream apps subscribe to.
    static EVENTS: EventChannel<TrayIconEvent> = EventChannel::new();
    /// Send-safe bridge: tray-icon's handler (any thread) -> main-thread frame.
    static BRIDGE: (Sender<TrayIconEvent>, Receiver<TrayIconEvent>) = unbounded();
}

/// Register the system tray icon. Call BEFORE [`launch`](crate::launch);
/// Sigil creates it on the first frame and routes icon clicks to [`events`].
pub fn set_tray(builder: TrayIconBuilder) {
    PENDING.with(|slot| *slot.borrow_mut() = Some(builder));
}

/// Reactive stream of tray-icon clicks (left / right / double).
#[must_use]
pub fn events() -> EventChannel<TrayIconEvent> {
    EVENTS.with(Clone::clone)
}

/// Simulate a tray-icon event, as if the OS delivered it. A subscriber on
/// [`events`] fires synchronously — the test seam for tray handling, no
/// system tray or display required.
pub fn simulate(event: TrayIconEvent) {
    events().emit(event);
}

/// True while a tray icon is registered but not yet built (first-frame gate).
pub(crate) fn has_pending() -> bool {
    PENDING.with(|slot| slot.borrow().is_some())
}

/// Build the registered tray icon and bridge its events. `wake` wakes the
/// event loop when a tray event fires.
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

/// Drain bridged tray events into the reactive channel. Call each frame,
/// inside the effect phase.
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
