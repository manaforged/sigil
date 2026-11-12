use crate::events::Modifiers;
use crate::geometry::{Point, Size};

/// Pointer device.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum PointerType {
    /// The mouse pointer kind.
    #[default]
    Mouse,
    /// The touch pointer kind.
    Touch,
    /// The pen pointer kind.
    Pen,
}

/// One mouse button.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum MouseButton {
    /// The left mouse button.
    Left,
    /// The right mouse button.
    Right,
    /// The middle mouse button.
    Middle,
    /// The back mouse button.
    Back,
    /// The forward mouse button.
    Forward,
    /// The other mouse button.
    Other(u16),
}

/// Set of pressed buttons.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct MouseButtons(u8);

impl MouseButtons {
    /// No buttons.
    pub const EMPTY: Self = Self(0);
    /// The left button.
    pub const LEFT: Self = Self(1 << 0);
    /// The right button.
    pub const RIGHT: Self = Self(1 << 1);
    /// The middle button.
    pub const MIDDLE: Self = Self(1 << 2);
    /// The back button.
    pub const BACK: Self = Self(1 << 3);
    /// The forward button.
    pub const FORWARD: Self = Self(1 << 4);

    /// A set with no mouse buttons.
    #[must_use]
    pub const fn empty() -> Self {
        Self::EMPTY
    }

    /// Returns whether `button` is in this set.
    #[must_use]
    pub const fn contains(self, button: Self) -> bool {
        self.0 & button.0 == button.0
    }

    /// Adds `button` to this set.
    pub const fn insert(&mut self, button: Self) {
        self.0 |= button.0;
    }

    /// Removes `button` from this set.
    pub const fn remove(&mut self, button: Self) {
        self.0 &= !button.0;
    }
}

/// Pointer event phase.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum PointerPhase {
    /// The down pointer phase.
    Down,
    /// The move pointer phase.
    Move,
    /// The up pointer phase.
    Up,
    /// The enter pointer phase.
    Enter,
    /// The leave pointer phase.
    Leave,
    /// The cancel pointer phase.
    Cancel,
}

/// Pen tilt in degrees.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PointerTilt {
    /// The x.
    pub x: f32,
    /// The y.
    pub y: f32,
}

/// A pointer event.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct PointerEvent {
    /// The position.
    pub position: Point,
    /// The window position.
    pub window_position: Point,
    /// The delta.
    pub delta: Point,
    /// The kind.
    pub kind: PointerType,
    /// The button, if any.
    pub button: Option<MouseButton>,
    /// The buttons.
    pub buttons: MouseButtons,
    /// The modifiers.
    pub modifiers: Modifiers,
    /// The phase.
    pub phase: PointerPhase,
    /// The target size.
    pub target_size: Size,
    /// The pointer id.
    pub pointer_id: u64,
    /// The pressure, if any.
    pub pressure: Option<f32>,
    /// The tilt, if any.
    pub tilt: Option<PointerTilt>,
    capture_requested: bool,
    captured: bool,
    pub(super) propagation_stopped: bool,
}

impl PointerEvent {
    pub(crate) const fn new(phase: PointerPhase, position: Point) -> Self {
        Self {
            position,
            window_position: position,
            delta: Point::new(0.0, 0.0),
            kind: PointerType::Mouse,
            button: None,
            buttons: MouseButtons::empty(),
            modifiers: Modifiers::empty(),
            phase,
            target_size: Size::new(0.0, 0.0),
            pointer_id: 0,
            pressure: None,
            tilt: None,
            capture_requested: false,
            captured: false,
            propagation_stopped: false,
        }
    }

    #[cfg(any(test, feature = "desktop"))]
    #[must_use]
    pub(crate) const fn with_contact(
        mut self,
        pointer_id: u64,
        pressure: Option<f32>,
        tilt: Option<PointerTilt>,
    ) -> Self {
        self.pointer_id = pointer_id;
        self.pressure = pressure;
        self.tilt = tilt;
        self
    }

    /// Sends later pointer events to this node until release.
    pub const fn capture_pointer(&mut self) {
        self.capture_requested = true;
    }

    /// Returns whether this pointer event is captured.
    #[must_use]
    pub const fn is_captured(&self) -> bool {
        self.captured
    }

    /// Returns whether capture requested is set.
    #[must_use]
    pub(crate) const fn capture_requested(&self) -> bool {
        self.capture_requested
    }

    /// Sets the captured.
    pub(crate) const fn set_captured(&mut self, captured: bool) {
        self.captured = captured;
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ClickConfig {
    pub(crate) interval: core::time::Duration,
    pub(crate) distance: f32,
}

impl Default for ClickConfig {
    fn default() -> Self {
        Self {
            interval: core::time::Duration::from_millis(450),
            distance: 8.0,
        }
    }
}

thread_local! {
    static CLICK: core::cell::Cell<Option<ClickConfig>> = const { core::cell::Cell::new(None) };
}

impl ClickConfig {
    #[must_use]
    pub(crate) fn current() -> Self {
        CLICK.with(core::cell::Cell::get).unwrap_or_default()
    }

    #[cfg(app_host)]
    pub(crate) fn set(config: Self) {
        CLICK.with(|cell| cell.set(Some(config)));
    }

    #[must_use]
    pub(crate) fn is_repeat(&self, elapsed: core::time::Duration, from: Point, to: Point) -> bool {
        elapsed <= self.interval
            && (from.x - to.x).abs() <= self.distance
            && (from.y - to.y).abs() <= self.distance
    }
}
