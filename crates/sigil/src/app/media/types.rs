use core::time::Duration;

/// Whether the current track plays.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub enum PlaybackState {
    /// The stopped playback state.
    #[default]
    Stopped,
    /// The paused playback state.
    Paused,
    /// The playing playback state.
    Playing,
}

/// The repeat mode of a player.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub enum Repeat {
    /// The off repeat.
    #[default]
    Off,
    /// The one repeat.
    One,
    /// The all repeat.
    All,
}

/// The metadata of the current track.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub struct Track {
    /// The title.
    pub title: String,
    /// The artist, if any.
    pub artist: Option<String>,
    /// The album, if any.
    pub album: Option<String>,
    /// The cover url, if any.
    pub cover_url: Option<String>,
    /// The duration, if any.
    pub duration: Option<Duration>,
    /// The position, if any.
    pub position: Option<Duration>,
    /// The state.
    pub state: PlaybackState,
    /// Whether shuffle.
    pub shuffle: bool,
    /// The repeat.
    pub repeat: Repeat,
}

impl Track {
    /// Creates a track with `title` and no other metadata.
    #[must_use]
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            ..Self::default()
        }
    }

    /// Sets the name of the artist.
    #[must_use]
    pub fn artist(mut self, artist: impl Into<String>) -> Self {
        self.artist = Some(artist.into());
        self
    }

    /// Sets the name of the album.
    #[must_use]
    pub fn album(mut self, album: impl Into<String>) -> Self {
        self.album = Some(album.into());
        self
    }

    /// Sets the URL of the cover image.
    #[must_use]
    pub fn cover_url(mut self, url: impl Into<String>) -> Self {
        self.cover_url = Some(url.into());
        self
    }

    /// Sets the length of the track.
    #[must_use]
    pub const fn duration(mut self, duration: Duration) -> Self {
        self.duration = Some(duration);
        self
    }

    /// Sets the playback position in the track.
    #[must_use]
    pub const fn position(mut self, position: Duration) -> Self {
        self.position = Some(position);
        self
    }

    /// Sets the playback state of the track.
    #[must_use]
    pub const fn state(mut self, state: PlaybackState) -> Self {
        self.state = state;
        self
    }

    /// Sets whether shuffle is on.
    #[must_use]
    pub const fn shuffle(mut self, shuffle: bool) -> Self {
        self.shuffle = shuffle;
        self
    }

    /// Sets the repeat mode: off, one, or all.
    #[must_use]
    pub const fn repeat(mut self, repeat: Repeat) -> Self {
        self.repeat = repeat;
        self
    }
}

/// A press on a media key or control.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum MediaKey {
    /// The play media key.
    Play,
    /// The pause media key.
    Pause,
    /// The toggle media key.
    Toggle,
    /// The next media key.
    Next,
    /// The previous media key.
    Previous,
    /// The stop media key.
    Stop,
    /// The seek forward media key.
    SeekForward,
    /// The seek backward media key.
    SeekBackward,
    /// The seek by media key.
    SeekBy {
        /// Whether the seek moves forward.
        forward: bool,
        /// How far to seek.
        amount: Duration,
    },
    /// The set position media key.
    SetPosition(Duration),
    /// The set volume media key.
    SetVolume(f64),
    /// The set shuffle media key.
    SetShuffle(bool),
    /// The set repeat media key.
    SetRepeat(Repeat),
    /// The raise media key.
    Raise,
    /// The quit media key.
    Quit,
}
