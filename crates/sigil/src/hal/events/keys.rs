macro_rules! key_enum {
    ($(#[$meta:meta])* $name:ident { $($variant:ident),* $(,)? }) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        #[non_exhaustive]
        pub enum $name {
            $(#[doc = concat!("The `", stringify!($variant), "` key.")] $variant),*
        }

        impl $name {
            const ALL: &[Self] = &[$(Self::$variant),*];

            const fn name(self) -> &'static str {
                match self { $(Self::$variant => stringify!($variant)),* }
            }
        }

        impl core::fmt::Display for $name {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                f.write_str(self.name())
            }
        }

        impl core::str::FromStr for $name {
            type Err = crate::Error;

            fn from_str(text: &str) -> Result<Self, Self::Err> {
                Self::ALL
                    .iter()
                    .copied()
                    .find(|key| key.name() == text)
                    .ok_or_else(|| crate::Error::message(crate::ErrorKind::UnknownKey, text))
            }
        }
    };
}

key_enum! {
    /// A named key, by its meaning under the active layout.
    Key {
    Unidentified, AVRInput, AVRPower, Accept, Again, AllCandidates, Alphanumeric, Alt, AltGraph,
    AppSwitch, ArrowDown, ArrowLeft, ArrowRight, ArrowUp, Attn, AudioBalanceLeft,
    AudioBalanceRight, AudioBassBoostDown, AudioBassBoostToggle, AudioBassBoostUp,
    AudioFaderFront, AudioFaderRear, AudioSurroundModeNext, AudioTrebleDown, AudioTrebleUp,
    AudioVolumeDown, AudioVolumeMute, AudioVolumeUp, Backspace, BrightnessDown, BrightnessUp,
    BrowserBack, BrowserFavorites, BrowserForward, BrowserHome, BrowserRefresh, BrowserSearch,
    BrowserStop, Call, Camera, CameraFocus, Cancel, CapsLock, ChannelDown, ChannelUp, Clear,
    Close, ClosedCaptionToggle, CodeInput, ColorF0Red, ColorF1Green, ColorF2Yellow, ColorF3Blue,
    ColorF4Grey, ColorF5Brown, Compose, ContextMenu, Control, Convert, Copy, CrSel, Cut, DVR,
    Delete, Dimmer, DisplaySwap, Eisu, Eject, End, EndCall, Enter, EraseEof, Escape, ExSel,
    Execute, Exit, F1, F10, F11, F12, F13, F14, F15, F16, F17, F18, F19, F2, F20, F21, F22, F23,
    F24, F25, F26, F27, F28, F29, F3, F30, F31, F32, F33, F34, F35, F4, F5, F6, F7, F8, F9,
    FavoriteClear0, FavoriteClear1, FavoriteClear2, FavoriteClear3, FavoriteRecall0,
    FavoriteRecall1, FavoriteRecall2, FavoriteRecall3, FavoriteStore0, FavoriteStore1,
    FavoriteStore2, FavoriteStore3, FinalMode, Find, Fn, FnLock, GoBack, GoHome, GroupFirst,
    GroupLast, GroupNext, GroupPrevious, Guide, GuideNextDay, GuidePreviousDay, HangulMode,
    HanjaMode, Hankaku, HeadsetHook, Help, Hibernate, Hiragana, HiraganaKatakana, Home, Info,
    Insert, InstantReplay, JunjaMode, KanaMode, KanjiMode, Katakana, Key11, Key12,
    LastNumberRedial, LaunchApplication1, LaunchApplication2, LaunchCalendar, LaunchContacts,
    LaunchMail, LaunchMediaPlayer, LaunchMusicPlayer, LaunchPhone, LaunchScreenSaver,
    LaunchSpreadsheet, LaunchWebBrowser, LaunchWebCam, LaunchWordProcessor, Link, ListProgram,
    LiveContent, Lock, LogOff, MailForward, MailReply, MailSend, MannerMode, MediaApps,
    MediaAudioTrack, MediaClose, MediaFastForward, MediaLast, MediaPause, MediaPlay,
    MediaPlayPause, MediaRecord, MediaRewind, MediaSkipBackward, MediaSkipForward,
    MediaStepBackward, MediaStepForward, MediaStop, MediaTopMenu, MediaTrackNext,
    MediaTrackPrevious, Meta, MicrophoneToggle, MicrophoneVolumeDown, MicrophoneVolumeMute,
    MicrophoneVolumeUp, ModeChange, NavigateIn, NavigateNext, NavigateOut, NavigatePrevious,
    New, NextCandidate, NextFavoriteChannel, NextUserProfile, NonConvert, Notification, NumLock,
    OnDemand, Open, PageDown, PageUp, Pairing, Paste, Pause, PinPDown, PinPMove, PinPToggle,
    PinPUp, Play, PlaySpeedDown, PlaySpeedReset, PlaySpeedUp, Power, PowerOff,
    PreviousCandidate, Print, PrintScreen, Process, Props, RandomToggle, RcLowBattery,
    RecordSpeedNext, Redo, RfBypass, Romaji, STBInput, STBPower, Save, ScanChannelsToggle,
    ScreenModeNext, ScrollLock, Select, Settings, Shift, SingleCandidate, Soft1, Soft2, Soft3,
    Soft4, SpeechCorrectionList, SpeechInputToggle, SpellCheck, SplitScreenToggle, Standby,
    Subtitle, Symbol, SymbolLock, TV, TV3DMode, TVAntennaCable, TVAudioDescription,
    TVAudioDescriptionMixDown, TVAudioDescriptionMixUp, TVContentsMenu, TVDataService, TVInput,
    TVInputComponent1, TVInputComponent2, TVInputComposite1, TVInputComposite2, TVInputHDMI1,
    TVInputHDMI2, TVInputHDMI3, TVInputHDMI4, TVInputVGA1, TVMediaContext, TVNetwork,
    TVNumberEntry, TVPower, TVRadioService, TVSatellite, TVSatelliteBS, TVSatelliteCS,
    TVSatelliteToggle, TVTerrestrialAnalog, TVTerrestrialDigital, TVTimer, Tab, Teletext, Undo,
    VideoModeNext, VoiceDial, WakeUp, Wink, Zenkaku, ZenkakuHankaku, ZoomIn, ZoomOut,
    ZoomToggle,
    }
}

key_enum! {
    /// A key by its position on the keyboard.
    PhysicalKey {
    Unidentified, MetaLeft, MetaRight, Abort, Again, AltLeft, AltRight, ArrowDown, ArrowLeft,
    ArrowRight, ArrowUp, AudioVolumeDown, AudioVolumeMute, AudioVolumeUp, Backquote, Backslash,
    Backspace, BracketLeft, BracketRight, BrowserBack, BrowserFavorites, BrowserForward,
    BrowserHome, BrowserRefresh, BrowserSearch, BrowserStop, CapsLock, Comma, ContextMenu,
    ControlLeft, ControlRight, Convert, Copy, Cut, Delete, Digit0, Digit1, Digit2, Digit3,
    Digit4, Digit5, Digit6, Digit7, Digit8, Digit9, Eject, End, Enter, Equal, Escape, F1, F10,
    F11, F12, F13, F14, F15, F16, F17, F18, F19, F2, F20, F21, F22, F23, F24, F25, F26, F27,
    F28, F29, F3, F30, F31, F32, F33, F34, F35, F4, F5, F6, F7, F8, F9, Find, Fn, FnLock, Help,
    Hiragana, Home, Insert, IntlBackslash, IntlRo, IntlYen, KanaMode, Katakana, KeyA, KeyB,
    KeyC, KeyD, KeyE, KeyF, KeyG, KeyH, KeyI, KeyJ, KeyK, KeyL, KeyM, KeyN, KeyO, KeyP, KeyQ,
    KeyR, KeyS, KeyT, KeyU, KeyV, KeyW, KeyX, KeyY, KeyZ, Lang1, Lang2, Lang3, Lang4, Lang5,
    LaunchApp1, LaunchApp2, LaunchMail, MediaPlayPause, MediaSelect, MediaStop, MediaTrackNext,
    MediaTrackPrevious, Minus, NonConvert, NumLock, Numpad0, Numpad1, Numpad2, Numpad3, Numpad4,
    Numpad5, Numpad6, Numpad7, Numpad8, Numpad9, NumpadAdd, NumpadBackspace, NumpadClear,
    NumpadClearEntry, NumpadComma, NumpadDecimal, NumpadDivide, NumpadEnter, NumpadEqual,
    NumpadHash, NumpadMemoryAdd, NumpadMemoryClear, NumpadMemoryRecall, NumpadMemoryStore,
    NumpadMemorySubtract, NumpadMultiply, NumpadParenLeft, NumpadParenRight, NumpadStar,
    NumpadSubtract, Open, PageDown, PageUp, Paste, Pause, Period, Power, PrintScreen, Props,
    Quote, Resume, ScrollLock, Select, Semicolon, ShiftLeft, ShiftRight, Slash, Sleep, Space,
    Suspend, Tab, Undo, WakeUp,
    }
}

/// The modifier keys held during an input event.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct ModifiersState(u8);

impl ModifiersState {
    /// The shift key.
    pub const SHIFT: Self = Self(1);
    /// The control key.
    pub const CONTROL: Self = Self(1 << 1);
    /// The alt or option key.
    pub const ALT: Self = Self(1 << 2);
    /// The command, windows, or super key.
    pub const META: Self = Self(1 << 3);

    /// No modifiers.
    #[must_use]
    pub const fn empty() -> Self {
        Self(0)
    }

    /// True when no modifier is held.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// True when every modifier in `other` is held.
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// True when any modifier in `other` is held.
    #[must_use]
    pub const fn intersects(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }

    /// Both sets of modifiers.
    #[must_use]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// The modifiers in `self` that are not in `other`.
    #[must_use]
    pub const fn difference(self, other: Self) -> Self {
        Self(self.0 & !other.0)
    }

    /// The modifiers held in both sets.
    #[must_use]
    pub const fn intersection(self, other: Self) -> Self {
        Self(self.0 & other.0)
    }

    /// True when shift is held.
    #[must_use]
    pub const fn shift(self) -> bool {
        self.contains(Self::SHIFT)
    }

    /// True when control is held.
    #[must_use]
    pub const fn ctrl(self) -> bool {
        self.contains(Self::CONTROL)
    }

    /// True when alt is held.
    #[must_use]
    pub const fn alt(self) -> bool {
        self.contains(Self::ALT)
    }

    /// True when meta is held.
    #[must_use]
    pub const fn meta(self) -> bool {
        self.contains(Self::META)
    }

    /// Adds or removes `other`.
    pub const fn set(&mut self, other: Self, held: bool) {
        self.0 = if held {
            self.0 | other.0
        } else {
            self.0 & !other.0
        };
    }

    /// Adds `other`.
    pub const fn insert(&mut self, other: Self) {
        self.set(other, true);
    }

    /// Removes `other`.
    pub const fn remove(&mut self, other: Self) {
        self.set(other, false);
    }
}

impl core::ops::BitOr for ModifiersState {
    type Output = Self;

    fn bitor(self, other: Self) -> Self {
        self.union(other)
    }
}

impl core::ops::BitOrAssign for ModifiersState {
    fn bitor_assign(&mut self, other: Self) {
        self.insert(other);
    }
}

impl core::ops::Sub for ModifiersState {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        self.difference(other)
    }
}

impl core::ops::SubAssign for ModifiersState {
    fn sub_assign(&mut self, other: Self) {
        self.remove(other);
    }
}
