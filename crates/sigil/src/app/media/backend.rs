use core::time::Duration;

use playwire::{Event, MediaControls, PlayerConfig};

use super::types::{MediaKey, PlaybackState, Repeat, Track};
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
    sink: impl Fn(MediaKey) + Send + Sync + 'static,
) -> AppResult<MediaControls> {
    let bus_name: String = name
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect();
    let mut config = PlayerConfig::new(name);
    config.bus_name = bus_name;
    config.hwnd = hwnd.and_then(|raw| u64::try_from(raw.cast_unsigned()).ok());
    MediaControls::new(config, move |event| {
        if let Some(key) = to_key(&event) {
            sink(key);
        }
    })
    .map_err(|err| backend_error(&err))
}

pub(super) fn apply(controls: &mut MediaControls, track: &Track) -> AppResult<()> {
    controls
        .set_state(&to_state(track))
        .map_err(|err| backend_error(&err))
}

fn to_state(track: &Track) -> playwire::PlaybackState {
    let published = (track.state != PlaybackState::Stopped).then(|| playwire::Track {
        title: track.title.clone(),
        artists: track.artist.iter().cloned().collect(),
        album: track.album.clone().unwrap_or_default(),
        artwork_url: track.cover_url.clone().unwrap_or_default(),
        ..playwire::Track::default()
    });
    playwire::PlaybackState {
        track: published,
        playing: track.state == PlaybackState::Playing,
        position: track.position.unwrap_or_default(),
        duration: track.duration,
        volume: 1.0,
        repeat: to_repeat(track.repeat),
        shuffle: track.shuffle,
        ..playwire::PlaybackState::default()
    }
}

const fn to_repeat(repeat: Repeat) -> playwire::Repeat {
    match repeat {
        Repeat::Off => playwire::Repeat::Off,
        Repeat::One => playwire::Repeat::One,
        Repeat::All => playwire::Repeat::All,
    }
}

const fn from_repeat(repeat: playwire::Repeat) -> Repeat {
    match repeat {
        playwire::Repeat::Off => Repeat::Off,
        playwire::Repeat::One => Repeat::One,
        playwire::Repeat::All => Repeat::All,
    }
}

fn to_key(event: &Event) -> Option<MediaKey> {
    Some(match *event {
        Event::Play => MediaKey::Play,
        Event::Pause => MediaKey::Pause,
        Event::PlayPause => MediaKey::Toggle,
        Event::Next => MediaKey::Next,
        Event::Previous => MediaKey::Previous,
        Event::Stop => MediaKey::Stop,
        Event::SeekBy(seconds) => MediaKey::SeekBy {
            forward: seconds >= 0.0,
            amount: Duration::try_from_secs_f64(seconds.abs()).ok()?,
        },
        Event::SeekTo(at) => MediaKey::SetPosition(at),
        Event::SetVolume(volume) => MediaKey::SetVolume(volume),
        Event::SetShuffle(on) => MediaKey::SetShuffle(on),
        Event::SetRepeat(repeat) => MediaKey::SetRepeat(from_repeat(repeat)),
        Event::Raise => MediaKey::Raise,
        Event::Quit => MediaKey::Quit,
        _ => return None,
    })
}

fn backend_error(err: &playwire::Error) -> Error {
    Error::new(std::io::Error::other(err.to_string()))
}
