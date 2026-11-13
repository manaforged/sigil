use core::time::Duration;

/// Whether the current track is stopped, paused, or playing.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub enum PlaybackState {
    /// Nothing plays, and no track is published to the system's media controls.
    #[default]
    Stopped,
    /// A track is loaded and playback is halted at its position.
    Paused,
    /// A track is loaded and its position advances.
    Playing,
}

/// Which tracks a player plays again when they end.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub enum Repeat {
    /// Playback stops after the last track.
    #[default]
    Off,
    /// The current track plays again when it ends.
    One,
    /// The whole queue plays again when its last track ends.
    All,
}

/// The metadata and playback state of the current track, as the system's media controls show it.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub struct Track {
    pub(crate) title: String,
    pub(crate) artist: Option<String>,
    pub(crate) album: Option<String>,
    pub(crate) cover_url: Option<String>,
    pub(crate) duration: Option<Duration>,
    pub(crate) position: Option<Duration>,
    pub(crate) state: PlaybackState,
    pub(crate) shuffle: bool,
    pub(crate) repeat: Repeat,
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
    /// A request to start or resume playback.
    Play,
    /// A request to pause playback.
    Pause,
    /// A request to pause when playing and to play when paused.
    Toggle,
    /// A request to skip to the next track.
    Next,
    /// A request to go back to the previous track.
    Previous,
    /// A request to stop playback.
    Stop,
    /// A request to seek forward by a step that the app chooses.
    SeekForward,
    /// A request to seek backward by a step that the app chooses.
    SeekBackward,
    /// A request to seek by `amount` from the current position.
    SeekBy {
        /// True to seek forward, false to seek backward.
        forward: bool,
        /// How far to seek.
        amount: Duration,
    },
    /// A request to move playback to this position from the start of the track.
    SetPosition(Duration),
    /// A request to set the volume to this level.
    SetVolume(f64),
    /// A request to turn shuffle on with `true` or off with `false`.
    SetShuffle(bool),
    /// A request to change the repeat mode.
    SetRepeat(Repeat),
    /// A request to bring the app's window to the front.
    Raise,
    /// A request to quit the app.
    Quit,
}
