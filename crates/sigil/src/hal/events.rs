use crate::geometry::{PhysicalSize, Point, Scale};

mod build;
mod gesture;
mod keys;
mod pointer;
mod shortcut_parse;

pub use gesture::{Gesture, GestureEvent};
pub use keys::{Modifiers, NamedKey, PhysicalKey};
#[cfg(feature = "desktop")]
pub(crate) use keys::{for_each_named_key, for_each_physical_key};

pub(crate) use pointer::ClickConfig;
pub use pointer::{
    MouseButton, MouseButtons, PointerEvent, PointerPhase, PointerTilt, PointerType,
};

/// An input event from the host.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum Event {
    /// The pointer event.
    Pointer(PointerEvent),
    /// The key event.
    Key(KeyEvent),
    /// The ime event.
    Ime(ImeEvent),
    /// The scroll event.
    Scroll(ScrollEvent),
    /// The gesture event.
    Gesture(GestureEvent),
    /// The file drop event.
    FileDrop(FileDropEvent),
    /// The paste event.
    Paste(alloc::string::String),
    /// The window event.
    Window(WindowEvent),
}

/// File drop phase.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum FileDropPhase {
    /// The hovered file drop phase.
    Hovered,
    /// The dropped file drop phase.
    Dropped,
    /// The cancelled file drop phase.
    Cancelled,
}

/// A file drag over or onto the window.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct FileDropEvent {
    /// The path, if any.
    pub path: Option<alloc::string::String>,
    /// The phase.
    pub phase: FileDropPhase,
}

/// A named key or a character.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum Key {
    /// The named logical key.
    Named(NamedKey),
    /// The character logical key.
    Character(String),
}

impl Key {
    /// Creates a key from a `NamedKey`.
    #[must_use]
    pub const fn named(key: NamedKey) -> Self {
        Self::Named(key)
    }

    /// Creates a key from the text that it types.
    #[must_use]
    pub fn character(value: impl Into<String>) -> Self {
        Self::Character(value.into())
    }
}

impl From<NamedKey> for Key {
    fn from(value: NamedKey) -> Self {
        Self::Named(value)
    }
}

impl From<String> for Key {
    fn from(value: String) -> Self {
        Self::Character(value)
    }
}

#[cfg(target_os = "macos")]
const PRIMARY: Modifiers = Modifiers::META;
#[cfg(not(target_os = "macos"))]
const PRIMARY: Modifiers = Modifiers::CTRL;

const MODIFIER_LABELS: [(Modifiers, &str, &str); 4] = [
    (Modifiers::CTRL, "⌃", "Ctrl"),
    (Modifiers::ALT, "⌥", "Alt"),
    (Modifiers::SHIFT, "⇧", "Shift"),
    (Modifiers::META, "⌘", "Super"),
];

const KEY_LABELS: [(NamedKey, &str, &str); 12] = [
    (NamedKey::Enter, "↵", "Enter"),
    (NamedKey::Escape, "Esc", "Esc"),
    (NamedKey::Tab, "⇥", "Tab"),
    (NamedKey::Backspace, "⌫", "Backspace"),
    (NamedKey::Delete, "⌦", "Del"),
    (NamedKey::ArrowUp, "↑", "↑"),
    (NamedKey::ArrowDown, "↓", "↓"),
    (NamedKey::ArrowLeft, "←", "←"),
    (NamedKey::ArrowRight, "→", "→"),
    (NamedKey::PageUp, "PgUp", "PgUp"),
    (NamedKey::PageDown, "PgDn", "PgDn"),
    (NamedKey::Home, "Home", "Home"),
];

/// A key plus modifiers.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Shortcut {
    /// The key.
    pub key: Key,
    /// The modifiers.
    pub modifiers: Modifiers,
}

impl Shortcut {
    /// Creates a shortcut from a key and the modifiers that go with it.
    #[must_use]
    pub fn new(key: impl Into<Key>, modifiers: Modifiers) -> Self {
        Self {
            key: key.into(),
            modifiers,
        }
    }

    /// Creates a shortcut for `key` with no modifiers.
    #[must_use]
    pub fn plain(key: impl Into<Key>) -> Self {
        Self::new(key, Modifiers::empty())
    }

    /// Creates a shortcut that needs Ctrl.
    #[must_use]
    pub fn ctrl(key: impl Into<Key>) -> Self {
        Self::new(key, Modifiers::CTRL)
    }

    /// Creates a shortcut that needs Command on macOS and Ctrl elsewhere.
    #[must_use]
    pub fn primary(key: impl Into<Key>) -> Self {
        Self::new(key, PRIMARY)
    }

    /// The display label, in macOS style on macOS and joined with `+` on other systems.
    #[must_use]
    pub fn label(&self) -> String {
        self.label_for(cfg!(target_os = "macos"))
    }

    /// Returns the display label in macOS or other style.
    #[must_use]
    pub fn label_for(&self, mac: bool) -> String {
        let mut parts: Vec<&str> = MODIFIER_LABELS
            .iter()
            .filter(|(flag, _, _)| self.modifiers.contains(*flag))
            .map(|(_, apple, other)| if mac { *apple } else { *other })
            .collect();
        let key = match &self.key {
            Key::Character(text) => text.to_uppercase(),
            Key::Named(named) => KEY_LABELS
                .iter()
                .find(|(candidate, _, _)| candidate == named)
                .map_or_else(
                    || named.to_string(),
                    |(_, apple, other)| (if mac { *apple } else { *other }).to_owned(),
                ),
        };
        parts.push(&key);
        parts.join(if mac { "" } else { "+" })
    }

    /// Returns whether `event` triggers this shortcut.
    #[must_use]
    pub fn matches(&self, event: &KeyEvent) -> bool {
        self.key_matches(&event.key)
            && normalize_modifiers(self.modifiers) == event.normalized_modifiers()
    }

    fn key_matches(&self, key: &Key) -> bool {
        match (&self.key, key) {
            (Key::Character(ours), Key::Character(theirs))
                if self.modifiers.contains(Modifiers::SHIFT) =>
            {
                ours.chars()
                    .flat_map(char::to_lowercase)
                    .eq(theirs.chars().flat_map(char::to_lowercase))
            }
            (ours, theirs) => ours == theirs,
        }
    }
}

impl From<NamedKey> for Shortcut {
    fn from(value: NamedKey) -> Self {
        Self::plain(value)
    }
}

impl From<Key> for Shortcut {
    fn from(value: Key) -> Self {
        Self::plain(value)
    }
}

/// Press or release.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum KeyPhase {
    /// The down key phase.
    Down,
    /// The up key phase.
    Up,
}

/// A key press or release.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct KeyEvent {
    /// The key.
    pub key: Key,
    /// The physical.
    pub physical: PhysicalKey,
    /// The modifiers.
    pub modifiers: Modifiers,
    /// Whether repeat.
    pub repeat: bool,
    /// The phase.
    pub phase: KeyPhase,
    /// The text, if any.
    pub text: Option<String>,
    propagation_stopped: bool,
}

impl KeyEvent {
    pub(crate) const fn new(key: Key, phase: KeyPhase) -> Self {
        Self {
            key,
            physical: PhysicalKey::Unidentified,
            modifiers: Modifiers::empty(),
            repeat: false,
            phase,
            text: None,
            propagation_stopped: false,
        }
    }

    /// The modifiers of the event, limited to Shift, Ctrl, Alt, and Meta.
    #[must_use]
    pub const fn normalized_modifiers(&self) -> Modifiers {
        normalize_modifiers(self.modifiers)
    }
}

/// Input method event.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ImeEvent {
    /// The enabled ime event.
    Enabled,
    /// The preedit ime event.
    Preedit(String, Option<(usize, usize)>),
    /// The commit ime event.
    Commit(String),
    /// The disabled ime event.
    Disabled,
}

/// Scroll gesture phase.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum ScrollPhase {
    /// The started scroll phase.
    Started,
    /// The moved scroll phase.
    Moved,
    /// The ended scroll phase.
    Ended,
    /// The cancelled scroll phase.
    Cancelled,
}

/// Scroll amount.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum ScrollDelta {
    /// The pixels scroll delta.
    Pixels(Point),
    /// The lines scroll delta.
    Lines(Point),
}

/// A wheel or trackpad scroll.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct ScrollEvent {
    /// The position.
    pub position: Point,
    /// The delta.
    pub delta: ScrollDelta,
    /// The modifiers.
    pub modifiers: Modifiers,
    /// The phase.
    pub phase: ScrollPhase,
    propagation_stopped: bool,
}

impl ScrollEvent {
    pub(crate) const fn new(position: Point, delta: ScrollDelta, phase: ScrollPhase) -> Self {
        Self {
            position,
            delta,
            modifiers: Modifiers::empty(),
            phase,
            propagation_stopped: false,
        }
    }
}

macro_rules! propagation {
    ($($ty:ty),+) => {
        $(impl $ty {
            /// Stops the event from reaching ancestor nodes.
            pub const fn stop_propagation(&mut self) {
                self.propagation_stopped = true;
            }

            /// Returns whether propagation stopped is set.
            #[must_use]
            pub const fn propagation_stopped(&self) -> bool {
                self.propagation_stopped
            }
        })+
    };
}

propagation!(PointerEvent, KeyEvent, ScrollEvent, GestureEvent);

/// Resize, focus, and close events of the window.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum WindowEvent {
    /// The resized window event.
    Resized(PhysicalSize),
    /// The scale factor changed window event.
    ScaleFactorChanged(Scale),
    /// The focused window event.
    Focused(bool),
    /// The close requested window event.
    CloseRequested,
    /// The destroyed window event.
    Destroyed,
    /// The occluded window event.
    Occluded(bool),
}

const fn normalize_modifiers(modifiers: Modifiers) -> Modifiers {
    modifiers.intersection(
        Modifiers::SHIFT
            .union(Modifiers::CTRL)
            .union(Modifiers::ALT)
            .union(Modifiers::META),
    )
}
