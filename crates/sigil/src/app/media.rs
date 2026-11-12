use crate::app::Wake;
use crate::app::bridge::Native;
use crate::error::AppResult;
use crate::reactive::EventChannel;
use backend::{Controls, apply, create};

#[cfg_attr(
    not(any(
        vendor_os,
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
    static NATIVE: Native<Track, Controls, MediaKey> = Native::new();
}

/// The channel that delivers media key presses.
#[must_use]
pub fn events() -> EventChannel<MediaKey> {
    NATIVE.with(Native::events)
}

#[cfg(feature = "kittest")]
pub fn simulate(key: MediaKey) {
    events().emit(key);
}

/// Publishes `track` to the now-playing controls.
pub fn now_playing(track: Track) -> AppResult<()> {
    backend::supported()?;
    NATIVE.with(|native| {
        let applied =
            native.installed(|slot| slot.as_mut().map(|controls| apply(controls, &track)));
        applied.unwrap_or_else(|| {
            native.queue(track);
            Ok(())
        })
    })
}

fn is_installed() -> bool {
    NATIVE.with(|native| native.installed(|slot| slot.is_some()))
}

pub(crate) fn install(title: &str, hwnd: Option<isize>, wake: impl FnOnce() -> Wake) {
    if !is_installed()
        && let Err(error) = install_controls(title, hwnd, wake())
    {
        tracing::warn!("sigil-app: cannot install media controls: {error}");
    }
}

pub(crate) fn pump() {
    NATIVE.with(Native::pump);
}

fn install_controls(name: &str, hwnd: Option<isize>, wake: Wake) -> AppResult<()> {
    NATIVE.with(|native| {
        let mut controls = create(name, hwnd, native.sink(wake))?;
        if let Some(track) = native.take() {
            apply(&mut controls, &track)?;
        }
        native.installed(|slot| *slot = Some(controls));
        Ok(())
    })
}
