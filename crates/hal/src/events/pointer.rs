//! Pointer and mouse-button event vocabulary.

use crate::{LogicalSize, ModifiersState, Point};

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
    /// Window-relative pointer position. Routing never rewrites this, so
    /// context menus and other overlays can anchor in viewport space.
    pub window_position: Point,
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
    /// Border-box size of the node currently handling this event, in the
    /// same logical pixels as [`Self::position`].
    ///
    /// Handlers that turn a pointer position into a ratio (slider tracks,
    /// scrollbar thumbs) must divide by this rather than by a
    /// compile-time constant, or they silently mis-map input the moment
    /// the widget is any size other than its default.
    ///
    /// Zero when the event has not been routed through a laid-out node
    /// (synthesized events, direct handler invocation) — treat a zero
    /// extent as "no ratio available" rather than dividing by it.
    pub target_size: LogicalSize,
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
            window_position: init.position,
            delta: init.delta,
            kind: init.kind,
            button: init.button,
            buttons: init.buttons,
            modifiers: init.modifiers,
            phase: init.phase,
            // Populated by the router once the target node is known.
            target_size: LogicalSize::new(0.0, 0.0),
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
