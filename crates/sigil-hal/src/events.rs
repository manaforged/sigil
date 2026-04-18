//! Shared input and window event vocabulary.
//!
//! Platform adapters translate their native event types into these
//! structs before higher layers route them through the composed node
//! tree. No `winit` types cross this boundary.

use smol_str::SmolStr;

use crate::{PhysicalSize, Point, Scale};

pub use keyboard_types::{Code as PhysicalKey, Modifiers as ModifiersState, NamedKey as Key};

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

/// Pointer device family.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum PointerKind {
    /// Mouse or trackpad pointer.
    #[default]
    Mouse,
    /// Touchscreen contact.
    Touch,
    /// Pen or stylus contact.
    Pen,
}

/// Mouse button identity.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum MouseButton {
    /// Primary button.
    Left,
    /// Secondary button.
    Right,
    /// Middle button.
    Middle,
    /// Browser back button.
    Back,
    /// Browser forward button.
    Forward,
    /// Adapter-specific button.
    Other(u16),
}

/// Set of currently pressed mouse buttons.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct MouseButtons(u8);

impl MouseButtons {
    /// No buttons pressed.
    pub const EMPTY: Self = Self(0);
    /// Left button bit.
    pub const LEFT: Self = Self(1 << 0);
    /// Right button bit.
    pub const RIGHT: Self = Self(1 << 1);
    /// Middle button bit.
    pub const MIDDLE: Self = Self(1 << 2);
    /// Back button bit.
    pub const BACK: Self = Self(1 << 3);
    /// Forward button bit.
    pub const FORWARD: Self = Self(1 << 4);

    /// Return an empty set.
    #[must_use]
    pub const fn empty() -> Self {
        Self::EMPTY
    }

    /// Return whether this set contains `button`.
    #[must_use]
    pub const fn contains(self, button: Self) -> bool {
        self.0 & button.0 == button.0
    }

    /// Insert a button bit.
    pub const fn insert(&mut self, button: Self) {
        self.0 |= button.0;
    }

    /// Remove a button bit.
    pub const fn remove(&mut self, button: Self) {
        self.0 &= !button.0;
    }
}

/// Pointer event phase.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PointerPhase {
    /// Button or contact pressed.
    Down,
    /// Pointer moved.
    Move,
    /// Button or contact released.
    Up,
    /// Pointer entered the window.
    Enter,
    /// Pointer left the window.
    Leave,
    /// Pointer stream was cancelled.
    Cancel,
}

/// Pointer input event.
#[derive(Clone, Debug, PartialEq)]
pub struct PointerEvent {
    /// Pointer position in logical pixels.
    ///
    /// Shell adapters initialise this as window-relative. During event
    /// routing, pointer handlers receive it in the current target node's
    /// local coordinate space.
    pub position: Point,
    /// Movement since the previous pointer event.
    pub delta: Point,
    /// Pointer device family.
    pub kind: PointerKind,
    /// Button associated with this event, when any.
    pub button: Option<MouseButton>,
    /// All currently pressed mouse buttons.
    pub buttons: MouseButtons,
    /// Active keyboard modifiers.
    pub modifiers: ModifiersState,
    /// Event phase.
    pub phase: PointerPhase,
    capture_requested: bool,
    captured: bool,
    propagation_stopped: bool,
}

/// Initial values for a [`PointerEvent`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PointerEventInit {
    /// Window-relative position.
    pub position: Point,
    /// Movement since the previous pointer event.
    pub delta: Point,
    /// Pointer device family.
    pub kind: PointerKind,
    /// Button associated with this event, when any.
    pub button: Option<MouseButton>,
    /// All currently pressed mouse buttons.
    pub buttons: MouseButtons,
    /// Active keyboard modifiers.
    pub modifiers: ModifiersState,
    /// Event phase.
    pub phase: PointerPhase,
}

impl PointerEvent {
    /// Construct a pointer event.
    #[must_use]
    pub const fn new(init: PointerEventInit) -> Self {
        Self {
            position: init.position,
            delta: init.delta,
            kind: init.kind,
            button: init.button,
            buttons: init.buttons,
            modifiers: init.modifiers,
            phase: init.phase,
            capture_requested: false,
            captured: false,
            propagation_stopped: false,
        }
    }

    /// Stop routing this event to later phases.
    pub const fn stop_propagation(&mut self) {
        self.propagation_stopped = true;
    }

    /// Request pointer capture for the current target node.
    pub const fn capture_pointer(&mut self) {
        self.capture_requested = true;
    }

    /// Return whether the router is delivering this event through pointer capture.
    #[must_use]
    pub const fn is_captured(&self) -> bool {
        self.captured
    }

    /// Return whether propagation was stopped.
    #[must_use]
    pub const fn propagation_stopped(&self) -> bool {
        self.propagation_stopped
    }

    /// Return whether pointer capture was requested.
    #[must_use]
    pub const fn capture_requested(&self) -> bool {
        self.capture_requested
    }

    /// Mark this event as capture-delivered.
    #[doc(hidden)]
    pub const fn set_captured(&mut self, captured: bool) {
        self.captured = captured;
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
