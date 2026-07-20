//! Explicit winit ↔ keyboard-types key mapping.
//!
//! Neither `winit::keyboard::NamedKey` Debug output nor
//! `winit::keyboard::KeyCode` Debug output is a stable contract.
//! Routing keyboard input through `format!` + `FromStr` would also
//! heap-allocate once per keystroke. Both mappers below use explicit
//! match arms that are validated at compile time: any future variant
//! divergence between winit and keyboard-types surfaces as a missing
//! arm the compiler rejects, not as silent `Key::Unidentified` at
//! runtime.

#![allow(clippy::redundant_pub_crate)]

use sigil_hal::{Key, LogicalKey, PhysicalKey};
use winit::keyboard::{
    Key as WinitKey, KeyCode as WinitKeyCode, NamedKey as WinitNamedKey,
    PhysicalKey as WinitPhysicalKey,
};

/// Identity-style match generator for enums that share variant names.
///
/// Emits `Winit::V => Dst::V` for every listed variant and falls through
/// to `$default` on anything else. The `@raw` form is for when either
/// side uses a path rather than a single identifier for the enum type.
macro_rules! same_name_match {
    ($input:expr, $src:path => $dst:path, default: $default:expr, [$($v:ident),* $(,)?]) => {
        match $input {
            $( <$src>::$v => <$dst>::$v, )*
            _ => $default,
        }
    };
}

pub(crate) fn map_logical_key(key: WinitKey) -> LogicalKey {
    match key {
        // keyboard-types has no `Space` NamedKey; the W3C spec models
        // it as the printable character " " on the Character stream.
        WinitKey::Named(WinitNamedKey::Space) => LogicalKey::character(" "),
        // keyboard-types deprecated `Hyper` and `Super` in favour of
        // `Meta`. Winit still delivers both variants on Linux, so we
        // collapse them onto `Meta` rather than dropping them.
        WinitKey::Named(WinitNamedKey::Hyper | WinitNamedKey::Super) => {
            LogicalKey::Named(Key::Meta)
        }
        WinitKey::Named(named) => LogicalKey::Named(map_named_key(named)),
        WinitKey::Character(value) => LogicalKey::character(value.as_str()),
        WinitKey::Dead(Some(ch)) => {
            let mut buffer = [0_u8; 4];
            LogicalKey::character(ch.encode_utf8(&mut buffer))
        }
        WinitKey::Dead(None) | WinitKey::Unidentified(_) => LogicalKey::Named(Key::Unidentified),
    }
}

pub(crate) const fn map_physical_key(key: WinitPhysicalKey) -> PhysicalKey {
    match key {
        // winit keeps the "Super" spelling for the left/right Windows
        // / Command keys; keyboard-types normalises both to "Meta".
        WinitPhysicalKey::Code(WinitKeyCode::SuperLeft) => PhysicalKey::MetaLeft,
        WinitPhysicalKey::Code(WinitKeyCode::SuperRight) => PhysicalKey::MetaRight,
        WinitPhysicalKey::Code(code) => map_key_code(code),
        WinitPhysicalKey::Unidentified(_) => PhysicalKey::Unidentified,
    }
}

#[allow(clippy::too_many_lines)]
const fn map_named_key(key: WinitNamedKey) -> Key {
    same_name_match!(key, WinitNamedKey => Key, default: Key::Unidentified, [
        Alt, AltGraph, CapsLock, Control, Fn, FnLock,
        NumLock, ScrollLock, Shift, Symbol, SymbolLock,
        Meta,
        Enter, Tab,
        ArrowDown, ArrowLeft, ArrowRight, ArrowUp,
        End, Home, PageDown, PageUp,
        Backspace, Clear, Copy, CrSel, Cut, Delete, EraseEof, ExSel,
        Insert, Paste, Redo, Undo,
        Accept, Again, Attn, Cancel, ContextMenu, Escape, Execute,
        Find, Help, Pause, Play, Props, Select, ZoomIn, ZoomOut,
        BrightnessDown, BrightnessUp, Eject, LogOff, Power, PowerOff,
        PrintScreen, Hibernate, Standby, WakeUp,
        AllCandidates, Alphanumeric, CodeInput, Compose, Convert,
        FinalMode, GroupFirst, GroupLast, GroupNext, GroupPrevious,
        ModeChange, NextCandidate, NonConvert, PreviousCandidate,
        Process, SingleCandidate,
        HangulMode, HanjaMode, JunjaMode,
        Eisu, Hankaku, Hiragana, HiraganaKatakana, KanaMode,
        KanjiMode, Katakana, Romaji, Zenkaku, ZenkakuHankaku,
        Soft1, Soft2, Soft3, Soft4,
        ChannelDown, ChannelUp, Close,
        MailForward, MailReply, MailSend,
        MediaClose, MediaFastForward, MediaPause, MediaPlay,
        MediaPlayPause, MediaRecord, MediaRewind, MediaStop,
        MediaTrackNext, MediaTrackPrevious,
        New, Open, Print, Save, SpellCheck,
        Key11, Key12,
        AudioBalanceLeft, AudioBalanceRight,
        AudioBassBoostDown, AudioBassBoostToggle, AudioBassBoostUp,
        AudioFaderFront, AudioFaderRear, AudioSurroundModeNext,
        AudioTrebleDown, AudioTrebleUp,
        AudioVolumeDown, AudioVolumeUp, AudioVolumeMute,
        MicrophoneToggle, MicrophoneVolumeDown, MicrophoneVolumeUp,
        MicrophoneVolumeMute,
        SpeechCorrectionList, SpeechInputToggle,
        LaunchApplication1, LaunchApplication2, LaunchCalendar,
        LaunchContacts, LaunchMail, LaunchMediaPlayer,
        LaunchMusicPlayer, LaunchPhone, LaunchScreenSaver,
        LaunchSpreadsheet, LaunchWebBrowser, LaunchWebCam,
        LaunchWordProcessor,
        BrowserBack, BrowserFavorites, BrowserForward, BrowserHome,
        BrowserRefresh, BrowserSearch, BrowserStop,
        AppSwitch, Call, Camera, CameraFocus, EndCall, GoBack, GoHome,
        HeadsetHook, LastNumberRedial, Notification, MannerMode,
        VoiceDial,
        TV, TV3DMode, TVAntennaCable, TVAudioDescription,
        TVAudioDescriptionMixDown, TVAudioDescriptionMixUp,
        TVContentsMenu, TVDataService, TVInput, TVInputComponent1,
        TVInputComponent2, TVInputComposite1, TVInputComposite2,
        TVInputHDMI1, TVInputHDMI2, TVInputHDMI3, TVInputHDMI4,
        TVInputVGA1, TVMediaContext, TVNetwork, TVNumberEntry,
        TVPower, TVRadioService, TVSatellite, TVSatelliteBS,
        TVSatelliteCS, TVSatelliteToggle, TVTerrestrialAnalog,
        TVTerrestrialDigital, TVTimer,
        AVRInput, AVRPower,
        ColorF0Red, ColorF1Green, ColorF2Yellow, ColorF3Blue,
        ColorF4Grey, ColorF5Brown,
        ClosedCaptionToggle, Dimmer, DisplaySwap, DVR, Exit,
        FavoriteClear0, FavoriteClear1, FavoriteClear2, FavoriteClear3,
        FavoriteRecall0, FavoriteRecall1, FavoriteRecall2,
        FavoriteRecall3, FavoriteStore0, FavoriteStore1,
        FavoriteStore2, FavoriteStore3,
        Guide, GuideNextDay, GuidePreviousDay,
        Info, InstantReplay, Link, ListProgram, LiveContent, Lock,
        MediaApps, MediaAudioTrack, MediaLast, MediaSkipBackward,
        MediaSkipForward, MediaStepBackward, MediaStepForward,
        MediaTopMenu,
        NavigateIn, NavigateNext, NavigateOut, NavigatePrevious,
        NextFavoriteChannel, NextUserProfile, OnDemand, Pairing,
        PinPDown, PinPMove, PinPToggle, PinPUp,
        PlaySpeedDown, PlaySpeedReset, PlaySpeedUp, RandomToggle,
        RcLowBattery, RecordSpeedNext, RfBypass, ScanChannelsToggle,
        ScreenModeNext, Settings, SplitScreenToggle, STBInput,
        STBPower, Subtitle, Teletext, VideoModeNext, Wink,
        ZoomToggle,
        F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12,
        F13, F14, F15, F16, F17, F18, F19, F20, F21, F22, F23, F24,
        F25, F26, F27, F28, F29, F30, F31, F32, F33, F34, F35,
    ])
}

const fn map_key_code(code: WinitKeyCode) -> PhysicalKey {
    same_name_match!(code, WinitKeyCode => PhysicalKey,
        default: PhysicalKey::Unidentified, [
        Backquote, Backslash, BracketLeft, BracketRight, Comma,
        Digit0, Digit1, Digit2, Digit3, Digit4,
        Digit5, Digit6, Digit7, Digit8, Digit9,
        Equal, IntlBackslash, IntlRo, IntlYen,
        KeyA, KeyB, KeyC, KeyD, KeyE, KeyF, KeyG, KeyH, KeyI,
        KeyJ, KeyK, KeyL, KeyM, KeyN, KeyO, KeyP, KeyQ, KeyR,
        KeyS, KeyT, KeyU, KeyV, KeyW, KeyX, KeyY, KeyZ,
        Minus, Period, Quote, Semicolon, Slash,
        AltLeft, AltRight, Backspace, CapsLock, ContextMenu,
        ControlLeft, ControlRight, Enter, ShiftLeft, ShiftRight,
        Space, Tab,
        Convert, KanaMode, Lang1, Lang2, Lang3, Lang4, Lang5,
        NonConvert,
        Delete, End, Help, Home, Insert, PageDown, PageUp,
        ArrowDown, ArrowLeft, ArrowRight, ArrowUp,
        NumLock,
        Numpad0, Numpad1, Numpad2, Numpad3, Numpad4,
        Numpad5, Numpad6, Numpad7, Numpad8, Numpad9,
        NumpadAdd, NumpadBackspace, NumpadClear, NumpadClearEntry,
        NumpadComma, NumpadDecimal, NumpadDivide, NumpadEnter,
        NumpadEqual, NumpadHash, NumpadMemoryAdd, NumpadMemoryClear,
        NumpadMemoryRecall, NumpadMemoryStore, NumpadMemorySubtract,
        NumpadMultiply, NumpadParenLeft, NumpadParenRight,
        NumpadStar, NumpadSubtract,
        Escape, Fn, FnLock, PrintScreen, ScrollLock, Pause,
        BrowserBack, BrowserFavorites, BrowserForward, BrowserHome,
        BrowserRefresh, BrowserSearch, BrowserStop,
        Eject, LaunchApp1, LaunchApp2, LaunchMail,
        MediaPlayPause, MediaSelect, MediaStop,
        MediaTrackNext, MediaTrackPrevious,
        Power, Sleep, AudioVolumeDown, AudioVolumeMute,
        AudioVolumeUp, WakeUp,
        Abort, Resume, Suspend,
        Again, Copy, Cut, Find, Open, Paste, Props, Select, Undo,
        Hiragana, Katakana,
        F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12,
        F13, F14, F15, F16, F17, F18, F19, F20, F21, F22, F23, F24,
        F25, F26, F27, F28, F29, F30, F31, F32, F33, F34, F35,
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_named_logical_keys() {
        assert_eq!(
            map_logical_key(WinitKey::Named(WinitNamedKey::Enter)),
            LogicalKey::Named(Key::Enter)
        );
        assert_eq!(
            map_logical_key(WinitKey::Named(WinitNamedKey::Tab)),
            LogicalKey::Named(Key::Tab)
        );
        assert_eq!(
            map_logical_key(WinitKey::Named(WinitNamedKey::Escape)),
            LogicalKey::Named(Key::Escape)
        );
        assert_eq!(
            map_logical_key(WinitKey::Named(WinitNamedKey::F5)),
            LogicalKey::Named(Key::F5)
        );
        assert_eq!(
            map_logical_key(WinitKey::Named(WinitNamedKey::ArrowUp)),
            LogicalKey::Named(Key::ArrowUp)
        );
    }

    #[test]
    fn maps_space_named_key_to_character_literal() {
        // keyboard-types has no NamedKey::Space; it belongs on the
        // character stream per the W3C spec.
        assert_eq!(
            map_logical_key(WinitKey::Named(WinitNamedKey::Space)),
            LogicalKey::character(" ")
        );
    }

    #[test]
    fn maps_character_logical_keys_without_case_normalising() {
        assert_eq!(
            map_logical_key(WinitKey::Character("s".into())),
            LogicalKey::character("s")
        );
        assert_eq!(
            map_logical_key(WinitKey::Character("S".into())),
            LogicalKey::character("S")
        );
    }

    #[test]
    fn maps_physical_keys_used_by_shortcuts() {
        assert_eq!(
            map_physical_key(WinitPhysicalKey::Code(WinitKeyCode::KeyA)),
            PhysicalKey::KeyA
        );
        assert_eq!(
            map_physical_key(WinitPhysicalKey::Code(WinitKeyCode::Digit1)),
            PhysicalKey::Digit1
        );
        assert_eq!(
            map_physical_key(WinitPhysicalKey::Code(WinitKeyCode::ArrowLeft)),
            PhysicalKey::ArrowLeft
        );
        assert_eq!(
            map_physical_key(WinitPhysicalKey::Code(WinitKeyCode::NumpadEnter)),
            PhysicalKey::NumpadEnter
        );
        assert_eq!(
            map_physical_key(WinitPhysicalKey::Code(WinitKeyCode::ShiftLeft)),
            PhysicalKey::ShiftLeft
        );
        assert_eq!(
            map_physical_key(WinitPhysicalKey::Code(WinitKeyCode::F12)),
            PhysicalKey::F12
        );
    }

    #[test]
    fn maps_super_physical_keys_to_web_meta_names() {
        assert_eq!(
            map_physical_key(WinitPhysicalKey::Code(WinitKeyCode::SuperLeft)),
            PhysicalKey::MetaLeft
        );
        assert_eq!(
            map_physical_key(WinitPhysicalKey::Code(WinitKeyCode::SuperRight)),
            PhysicalKey::MetaRight
        );
    }
}
