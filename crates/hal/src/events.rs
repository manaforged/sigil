//! Shared input and window event vocabulary.
//!
//! Platform adapters translate their native event types into these
//! structs before higher layers route them through the composed node
//! tree. No `winit` types cross this boundary.

use smol_str::SmolStr;

use crate::{PhysicalSize, Point, Scale};

pub use keyboard_types::{Code as PhysicalKey, Modifiers as ModifiersState, NamedKey as Key};

mod pointer;

pub use pointer::{
    MouseButton, MouseButtons, PointerEvent, PointerEventInit, PointerKind, PointerPhase,
};

/// High-level event delivered by a [`Shell`](crate::Shell).
#[derive(Clone, Debug, PartialEq)]
pub enum ShellEvent {
    /// Pointer input such as mouse move, down, up, enter, or leave.
    Pointer(PointerEvent),
    /// Keyboard input.
    Key(KeyEvent),
    /// Input-method composition event.
    Ime(ImeEvent),
    /// Scroll wheel or touchpad scroll event.
    Scroll(ScrollEvent),
    /// Window lifecycle or focus event.
    Window(WindowEvent),
}

/// Logical key used for event matching.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum LogicalKey {
    /// A named non-text key from `keyboard-types`.
    Named(Key),
    /// Text produced by a printable key.
    Character(SmolStr),
}

impl LogicalKey {
    /// Construct a named logical key.
    #[must_use]
    pub const fn named(key: Key) -> Self {
        Self::Named(key)
    }

    /// Construct a printable logical key.
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

/// Keyboard shortcut used by key handlers and accessibility semantics.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Shortcut {
    /// Key matched by this shortcut.
    pub key: LogicalKey,
    /// Required modifier state.
    pub modifiers: ModifiersState,
}

impl Shortcut {
    /// Construct a shortcut from a key and explicit modifiers.
    #[must_use]
    pub fn new(key: impl Into<LogicalKey>, modifiers: ModifiersState) -> Self {
        Self {
            key: key.into(),
            modifiers,
        }
    }

    /// Construct a shortcut without modifiers.
    #[must_use]
    pub fn plain(key: impl Into<LogicalKey>) -> Self {
        Self::new(key, ModifiersState::empty())
    }

    /// Construct a Control-modified shortcut.
    #[must_use]
    pub fn ctrl(key: impl Into<LogicalKey>) -> Self {
        Self::new(key, ModifiersState::CONTROL)
    }

    /// Return whether this shortcut matches a key event.
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

/// Keyboard event phase.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum KeyPhase {
    /// Key down.
    Down,
    /// Key up.
    Up,
}

/// Keyboard input event.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KeyEvent {
    /// Logical key used for shortcuts and handlers.
    pub key: LogicalKey,
    /// Physical key position.
    pub physical: PhysicalKey,
    /// Active keyboard modifiers.
    pub modifiers: ModifiersState,
    /// Whether this is an auto-repeat keydown.
    pub repeat: bool,
    /// Key phase.
    pub phase: KeyPhase,
    /// Printable text produced by this event, when any.
    pub text: Option<SmolStr>,
    propagation_stopped: bool,
}

/// Initial values for a [`KeyEvent`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KeyEventInit {
    /// Logical key used for shortcuts and handlers.
    pub key: LogicalKey,
    /// Physical key position.
    pub physical: PhysicalKey,
    /// Active keyboard modifiers.
    pub modifiers: ModifiersState,
    /// Whether this is an auto-repeat keydown.
    pub repeat: bool,
    /// Key phase.
    pub phase: KeyPhase,
    /// Printable text produced by this event, when any.
    pub text: Option<SmolStr>,
}

impl KeyEvent {
    /// Construct a key event.
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

    /// Stop routing this event to later phases.
    pub const fn stop_propagation(&mut self) {
        self.propagation_stopped = true;
    }

    /// Return whether propagation was stopped.
    #[must_use]
    pub const fn propagation_stopped(&self) -> bool {
        self.propagation_stopped
    }

    /// Return the four routing-relevant modifiers.
    #[must_use]
    pub const fn normalized_modifiers(&self) -> ModifiersState {
        normalize_modifiers(self.modifiers)
    }
}

/// Input-method event.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ImeEvent {
    /// IME composition enabled.
    Enabled,
    /// In-progress preedit text and cursor byte range.
    Preedit(SmolStr, Option<(usize, usize)>),
    /// Committed text.
    Commit(SmolStr),
    /// IME composition disabled.
    Disabled,
}

/// Scroll event phase.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ScrollPhase {
    /// Scroll stream started.
    Started,
    /// Scroll stream moved.
    Moved,
    /// Scroll stream ended.
    Ended,
    /// Scroll stream cancelled.
    Cancelled,
}

/// Normalised scroll delta.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ScrollDelta {
    /// Pixel delta.
    Pixels(Point),
    /// Line delta.
    Lines(Point),
}

/// Scroll input event.
#[derive(Clone, Debug, PartialEq)]
pub struct ScrollEvent {
    /// Window-relative pointer position at the time of the scroll.
    pub position: Point,
    /// Scroll movement.
    pub delta: ScrollDelta,
    /// Active keyboard modifiers.
    pub modifiers: ModifiersState,
    /// Scroll phase.
    pub phase: ScrollPhase,
}

/// Window lifecycle event.
#[derive(Clone, Debug, PartialEq)]
pub enum WindowEvent {
    /// Window resized.
    Resized(PhysicalSize),
    /// Window DPI scale changed.
    ScaleFactorChanged(Scale),
    /// Window focus changed.
    Focused(bool),
    /// Window close was requested.
    CloseRequested,
    /// Window was destroyed.
    Destroyed,
    /// Window occlusion changed.
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
