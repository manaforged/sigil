use smol_str::SmolStr;

use crate::geometry::{PhysicalSize, Point, Scale};

pub use keyboard_types::{Code as PhysicalKey, Modifiers as ModifiersState, NamedKey as Key};

mod gesture;
mod paste;
mod pointer;
mod shortcut_parse;

pub use gesture::{GestureEvent, GestureEventInit, GestureKind};
pub use paste::PasteEvent;
pub use shortcut_parse::ShortcutParseError;

pub use pointer::{
    ClickConfig, MouseButton, MouseButtons, PointerEvent, PointerEventInit, PointerKind,
    PointerPhase, PointerTilt,
};

#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum ShellEvent {
    Pointer(PointerEvent),
    Key(KeyEvent),
    Ime(ImeEvent),
    Scroll(ScrollEvent),
    Gesture(GestureEvent),
    FileDrop(FileDropEvent),
    Paste(alloc::string::String),
    Window(WindowEvent),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum FileDropPhase {
    Hovered,
    Dropped,
    Cancelled,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FileDropEvent {
    pub path: Option<alloc::string::String>,
    pub phase: FileDropPhase,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum LogicalKey {
    Named(Key),
    Character(SmolStr),
}

impl LogicalKey {
    #[must_use]
    pub const fn named(key: Key) -> Self {
        Self::Named(key)
    }

    #[must_use]
    pub fn character(value: impl Into<SmolStr>) -> Self {
        Self::Character(value.into())
    }
}

impl From<Key> for LogicalKey {
    fn from(value: Key) -> Self {
        Self::Named(value)
    }
}

impl From<SmolStr> for LogicalKey {
    fn from(value: SmolStr) -> Self {
        Self::Character(value)
    }
}

#[cfg(target_os = "macos")]
pub const PRIMARY: ModifiersState = ModifiersState::META;
#[cfg(not(target_os = "macos"))]
pub const PRIMARY: ModifiersState = ModifiersState::CONTROL;

const MODIFIER_LABELS: [(ModifiersState, &str, &str); 4] = [
    (ModifiersState::CONTROL, "⌃", "Ctrl"),
    (ModifiersState::ALT, "⌥", "Alt"),
    (ModifiersState::SHIFT, "⇧", "Shift"),
    (ModifiersState::META, "⌘", "Super"),
];

const KEY_LABELS: [(Key, &str, &str); 12] = [
    (Key::Enter, "↵", "Enter"),
    (Key::Escape, "Esc", "Esc"),
    (Key::Tab, "⇥", "Tab"),
    (Key::Backspace, "⌫", "Backspace"),
    (Key::Delete, "⌦", "Del"),
    (Key::ArrowUp, "↑", "↑"),
    (Key::ArrowDown, "↓", "↓"),
    (Key::ArrowLeft, "←", "←"),
    (Key::ArrowRight, "→", "→"),
    (Key::PageUp, "PgUp", "PgUp"),
    (Key::PageDown, "PgDn", "PgDn"),
    (Key::Home, "Home", "Home"),
];

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Shortcut {
    pub key: LogicalKey,
    pub modifiers: ModifiersState,
}

impl Shortcut {
    #[must_use]
    pub fn new(key: impl Into<LogicalKey>, modifiers: ModifiersState) -> Self {
        Self {
            key: key.into(),
            modifiers,
        }
    }

    #[must_use]
    pub fn plain(key: impl Into<LogicalKey>) -> Self {
        Self::new(key, ModifiersState::empty())
    }

    #[must_use]
    pub fn ctrl(key: impl Into<LogicalKey>) -> Self {
        Self::new(key, ModifiersState::CONTROL)
    }

    #[must_use]
    pub fn primary(key: impl Into<LogicalKey>) -> Self {
        Self::new(key, PRIMARY)
    }

    #[must_use]
    pub fn label(&self) -> String {
        self.label_for(cfg!(target_os = "macos"))
    }

    #[must_use]
    pub fn label_for(&self, mac: bool) -> String {
        let mut parts: Vec<&str> = MODIFIER_LABELS
            .iter()
            .filter(|(flag, _, _)| self.modifiers.contains(*flag))
            .map(|(_, apple, other)| if mac { *apple } else { *other })
            .collect();
        let key = match &self.key {
            LogicalKey::Character(text) => text.to_uppercase(),
            LogicalKey::Named(named) => KEY_LABELS
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

    #[must_use]
    pub fn matches(&self, event: &KeyEvent) -> bool {
        self.key == event.key && normalize_modifiers(self.modifiers) == event.normalized_modifiers()
    }
}

impl From<Key> for Shortcut {
    fn from(value: Key) -> Self {
        Self::plain(value)
    }
}

impl From<LogicalKey> for Shortcut {
    fn from(value: LogicalKey) -> Self {
        Self::plain(value)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum KeyPhase {
    Down,
    Up,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct KeyEvent {
    pub key: LogicalKey,
    pub physical: PhysicalKey,
    pub modifiers: ModifiersState,
    pub repeat: bool,
    pub phase: KeyPhase,
    pub text: Option<SmolStr>,
    propagation_stopped: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KeyEventInit {
    pub key: LogicalKey,
    pub physical: PhysicalKey,
    pub modifiers: ModifiersState,
    pub repeat: bool,
    pub phase: KeyPhase,
    pub text: Option<SmolStr>,
}

impl KeyEvent {
    #[must_use]
    pub fn new(init: KeyEventInit) -> Self {
        Self {
            key: init.key,
            physical: init.physical,
            modifiers: init.modifiers,
            repeat: init.repeat,
            phase: init.phase,
            text: init.text,
            propagation_stopped: false,
        }
    }

    pub const fn stop_propagation(&mut self) {
        self.propagation_stopped = true;
    }

    #[must_use]
    pub const fn propagation_stopped(&self) -> bool {
        self.propagation_stopped
    }

    #[must_use]
    pub const fn normalized_modifiers(&self) -> ModifiersState {
        normalize_modifiers(self.modifiers)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ImeEvent {
    Enabled,
    Preedit(SmolStr, Option<(usize, usize)>),
    Commit(SmolStr),
    Disabled,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum ScrollPhase {
    Started,
    Moved,
    Ended,
    Cancelled,
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum ScrollDelta {
    Pixels(Point),
    Lines(Point),
}

#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct ScrollEvent {
    pub position: Point,
    pub delta: ScrollDelta,
    pub modifiers: ModifiersState,
    pub phase: ScrollPhase,
    propagation_stopped: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrollEventInit {
    pub position: Point,
    pub delta: ScrollDelta,
    pub modifiers: ModifiersState,
    pub phase: ScrollPhase,
}

impl ScrollEvent {
    #[must_use]
    pub const fn new(init: ScrollEventInit) -> Self {
        Self {
            position: init.position,
            delta: init.delta,
            modifiers: init.modifiers,
            phase: init.phase,
            propagation_stopped: false,
        }
    }

    pub const fn stop_propagation(&mut self) {
        self.propagation_stopped = true;
    }

    #[must_use]
    pub const fn propagation_stopped(&self) -> bool {
        self.propagation_stopped
    }
}

#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum WindowEvent {
    Resized(PhysicalSize),
    ScaleFactorChanged(Scale),
    Focused(bool),
    CloseRequested,
    Destroyed,
    Occluded(bool),
}

const fn normalize_modifiers(modifiers: ModifiersState) -> ModifiersState {
    modifiers.intersection(
        ModifiersState::SHIFT
            .union(ModifiersState::CONTROL)
            .union(ModifiersState::ALT)
            .union(ModifiersState::META),
    )
}
