use std::cell::RefCell;

use crate::app::error::AppResult;
use crate::reactive::EventChannel;
use backend::{Controls, apply, create};
use crossbeam_channel::{Receiver, Sender, unbounded};

#[cfg_attr(
    not(any(
        target_os = "macos",
        target_os = "windows",
        target_os = "linux",
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "netbsd",
        target_os = "dragonfly"
    )),
    path = "media/unsupported.rs"
)]
mod backend;
mod types;

pub use types::{MediaKey, PlaybackState, Track};

thread_local! {
    static CONTROLS: RefCell<Option<Controls>> = const { RefCell::new(None) };
    static PENDING: RefCell<Option<Track>> = const { RefCell::new(None) };
    static EVENTS: EventChannel<MediaKey> = EventChannel::new();
    static BRIDGE: (Sender<MediaKey>, Receiver<MediaKey>) = unbounded();
}

#[must_use]
pub fn events() -> EventChannel<MediaKey> {
    EVENTS.with(Clone::clone)
}

#[cfg(any(test, feature = "kittest"))]
pub fn simulate(key: MediaKey) {
    events().emit(key);
}

pub fn now_playing(track: Track) -> AppResult<()> {
    backend::supported()?;
    let applied = CONTROLS.with(|slot| {
        slot.borrow_mut()
            .as_mut()
            .map(|controls| apply(controls, &track))
    });
    applied.unwrap_or_else(|| {
        PENDING.with(|slot| *slot.borrow_mut() = Some(track));
        Ok(())
    })
}

pub(crate) fn is_installed() -> bool {
    CONTROLS.with(|slot| slot.borrow().is_some())
}

pub(crate) fn install(
    name: &str,
    hwnd: Option<isize>,
    wake: impl Fn() + Send + Sync + 'static,
) -> AppResult<()> {
    if is_installed() {
        return Ok(());
    }
    let tx = BRIDGE.with(|(tx, _)| tx.clone());
    let mut controls = create(name, hwnd, move |key| {
        let _ = tx.send(key);
        wake();
    })?;
    if let Some(track) = PENDING.with(|slot| slot.borrow_mut().take()) {
        apply(&mut controls, &track)?;
    }
    CONTROLS.with(|slot| *slot.borrow_mut() = Some(controls));
    Ok(())
}

pub(crate) fn pump() {
    BRIDGE.with(|(_, rx)| {
        EVENTS.with(|channel| {
            while let Ok(key) = rx.try_recv() {
                channel.emit(key);
            }
        });
    });
}
