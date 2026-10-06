use core::ffi::c_void;

use souvlaki::{
    MediaControlEvent, MediaControls, MediaMetadata, MediaPlayback, MediaPosition, PlatformConfig,
    SeekDirection,
};

use super::types::{MediaKey, PlaybackState, Track};
use crate::app::error::{AppResult, Error};

pub(super) type Controls = MediaControls;

#[allow(
    clippy::unnecessary_wraps,
    reason = "matches the unsupported backend, which returns an error"
)]
pub(super) const fn supported() -> AppResult<()> {
    Ok(())
}

pub(super) fn create(
    name: &str,
    hwnd: Option<isize>,
    sink: impl Fn(MediaKey) + Send + 'static,
) -> AppResult<MediaControls> {
    let dbus_name: String = name
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect();
    let config = PlatformConfig {
        dbus_name: &dbus_name,
        display_name: name,
        hwnd: hwnd.map(|raw| std::ptr::with_exposed_provenance_mut::<c_void>(raw.cast_unsigned())),
    };
    let mut controls = MediaControls::new(config).map_err(|err| backend_error(&err))?;
    controls
        .attach(move |event| {
            if let Some(key) = to_key(&event) {
                sink(key);
            }
        })
        .map_err(|err| backend_error(&err))?;
    Ok(controls)
}

pub(super) fn apply(controls: &mut MediaControls, track: &Track) -> AppResult<()> {
    controls
        .set_metadata(MediaMetadata {
            title: Some(&track.title),
            album: track.album.as_deref(),
            artist: track.artist.as_deref(),
            cover_url: track.cover_url.as_deref(),
            duration: track.duration,
        })
        .map_err(|err| backend_error(&err))?;
    let progress = track.position.map(MediaPosition);
    let playback = match track.state {
        PlaybackState::Stopped => MediaPlayback::Stopped,
        PlaybackState::Paused => MediaPlayback::Paused { progress },
        PlaybackState::Playing => MediaPlayback::Playing { progress },
    };
    controls
        .set_playback(playback)
        .map_err(|err| backend_error(&err))
}

const fn to_key(event: &MediaControlEvent) -> Option<MediaKey> {
    Some(match *event {
        MediaControlEvent::Play => MediaKey::Play,
        MediaControlEvent::Pause => MediaKey::Pause,
        MediaControlEvent::Toggle => MediaKey::Toggle,
        MediaControlEvent::Next => MediaKey::Next,
        MediaControlEvent::Previous => MediaKey::Previous,
        MediaControlEvent::Stop => MediaKey::Stop,
        MediaControlEvent::Seek(SeekDirection::Forward) => MediaKey::SeekForward,
        MediaControlEvent::Seek(SeekDirection::Backward) => MediaKey::SeekBackward,
        MediaControlEvent::SeekBy(direction, amount) => MediaKey::SeekBy {
            forward: matches!(direction, SeekDirection::Forward),
            amount,
        },
        MediaControlEvent::SetPosition(MediaPosition(at)) => MediaKey::SetPosition(at),
        MediaControlEvent::SetVolume(volume) => MediaKey::SetVolume(volume),
        MediaControlEvent::Raise => MediaKey::Raise,
        MediaControlEvent::Quit => MediaKey::Quit,
        MediaControlEvent::OpenUri(_) => return None,
    })
}

fn backend_error(err: &souvlaki::Error) -> Error {
    Error::new(std::io::Error::other(format!("{err:?}")))
}
