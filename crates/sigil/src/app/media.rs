use std::cell::RefCell;

use crate::app::platform::{Bridge, Wake};
use crate::error::AppResult;
use crate::reactive::EventChannel;
use backend::{Controls, apply, create};

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

pub use types::{MediaKey, PlaybackState, Repeat, Track};

thread_local! {
    static CONTROLS: RefCell<Option<Controls>> = const { RefCell::new(None) };
    static PENDING: RefCell<Option<Track>> = const { RefCell::new(None) };
    static EVENTS: EventChannel<MediaKey> = EventChannel::new();
    static BRIDGE: Bridge<MediaKey> = Bridge::new();
}

/// Returns the channel of media key presses.
#[must_use]
pub fn events() -> EventChannel<MediaKey> {
    EVENTS.with(Clone::clone)
}

#[cfg(any(test, feature = "kittest"))]
pub fn simulate(key: MediaKey) {
    events().emit(key);
}

/// Publishes `track` to the now-playing controls.
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

fn is_installed() -> bool {
    CONTROLS.with(|slot| slot.borrow().is_some())
}

pub(crate) fn install(title: &str, hwnd: Option<isize>, wake: Wake) {
    if !is_installed()
        && let Err(error) = install_controls(title, hwnd, wake)
    {
        tracing::warn!("sigil-app: cannot install media controls: {error}");
    }
}

pub(crate) fn pump() {
    BRIDGE.with(|bridge| EVENTS.with(|channel| bridge.forward(channel)));
}

fn install_controls(name: &str, hwnd: Option<isize>, wake: Wake) -> AppResult<()> {
    let sink = BRIDGE.with(|bridge| bridge.sink(wake));
    let mut controls = create(name, hwnd, sink)?;
    if let Some(track) = PENDING.with(|slot| slot.borrow_mut().take()) {
        apply(&mut controls, &track)?;
    }
    CONTROLS.with(|slot| *slot.borrow_mut() = Some(controls));
    Ok(())
}
