use crate::events::Modifiers;
use crate::geometry::{Point, Size};

/// Pointer device.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum PointerType {
    /// A mouse; the default of a new pointer event.
    #[default]
    Mouse,
    /// A finger on a touch screen.
    Touch,
    /// A pen or stylus.
    Pen,
}

/// One mouse button.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum MouseButton {
    /// The left button, which is `MouseButtons::LEFT` in the held set.
    Left,
    /// The right button, which is `MouseButtons::RIGHT` in the held set.
    Right,
    /// The middle button, which is `MouseButtons::MIDDLE` in the held set.
    Middle,
    /// The back side button, which is `MouseButtons::BACK` in the held set.
    Back,
    /// The forward side button, which is `MouseButtons::FORWARD` in the held set.
    Forward,
    /// A button with no bit in `MouseButtons`, by the number that the host gives it.
    Other(u16),
}

/// Set of pressed buttons.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct MouseButtons(u8);

impl MouseButtons {
    /// Set while the left button is held.
    pub const LEFT: Self = Self(1 << 0);
    /// Set while the right button is held.
    pub const RIGHT: Self = Self(1 << 1);
    /// Set while the middle button is held.
    pub const MIDDLE: Self = Self(1 << 2);
    /// Set while the back side button is held.
    pub const BACK: Self = Self(1 << 3);
    /// Set while the forward side button is held.
    pub const FORWARD: Self = Self(1 << 4);

    /// A set with no mouse buttons.
    #[must_use]
    pub const fn empty() -> Self {
        Self(0)
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
    /// A button was pressed or a touch started.
    Down,
    /// The pointer moved.
    Move,
    /// A button was released or a touch ended.
    Up,
    /// The pointer came onto the node or the window.
    Enter,
    /// The pointer left the node or the window.
    Leave,
    /// The system cancelled the touch; pointer-up handlers run for it.
    Cancel,
}

/// Pen tilt in degrees.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct PointerTilt {
    pub(crate) x: f32,
    pub(crate) y: f32,
}

impl PointerTilt {
    /// Creates a pen tilt from x and y angles in degrees.
    #[must_use]
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    /// The tilt along the x axis, in degrees.
    #[must_use]
    pub const fn x(&self) -> f32 {
        self.x
    }

    /// The tilt along the y axis, in degrees.
    #[must_use]
    pub const fn y(&self) -> f32 {
        self.y
    }
}

/// A pointer event.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct PointerEvent {
    pub(crate) position: Point,
    pub(crate) window_position: Point,
    pub(crate) delta: Point,
    pub(crate) kind: PointerType,
    pub(crate) button: Option<MouseButton>,
    pub(crate) buttons: MouseButtons,
    pub(crate) modifiers: Modifiers,
    pub(crate) phase: PointerPhase,
    pub(crate) target_size: Size,
    pub(crate) pointer_id: u64,
    pub(crate) pressure: Option<f32>,
    pub(crate) tilt: Option<PointerTilt>,
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

    /// The pointer position from the top left corner of the handling node, in logical pixels.
    #[must_use]
    pub const fn position(&self) -> Point {
        self.position
    }

    /// The pointer position in the window, in logical pixels.
    #[must_use]
    pub const fn window_position(&self) -> Point {
        self.window_position
    }

    /// The movement since the last pointer position, in logical pixels.
    #[must_use]
    pub const fn delta(&self) -> Point {
        self.delta
    }

    /// The device: mouse, touch, or pen.
    #[must_use]
    pub const fn kind(&self) -> PointerType {
        self.kind
    }

    /// The button that was pressed or released; `None` when no button changed.
    #[must_use]
    pub const fn button(&self) -> Option<MouseButton> {
        self.button
    }

    /// The buttons held during the event.
    #[must_use]
    pub const fn buttons(&self) -> MouseButtons {
        self.buttons
    }

    /// The modifiers held during the event.
    #[must_use]
    pub const fn modifiers(&self) -> Modifiers {
        self.modifiers
    }

    /// What the pointer did: press, move, release, enter, leave, or cancel.
    #[must_use]
    pub const fn phase(&self) -> PointerPhase {
        self.phase
    }

    /// The size of the node that handles the event, in logical pixels.
    #[must_use]
    pub const fn target_size(&self) -> Size {
        self.target_size
    }

    /// The id of the touch point, which tells fingers apart; 0 for a mouse.
    #[must_use]
    pub const fn pointer_id(&self) -> u64 {
        self.pointer_id
    }

    /// The touch force from 0.0 to 1.0; `None` when the device reports none.
    #[must_use]
    pub const fn pressure(&self) -> Option<f32> {
        self.pressure
    }

    /// The pen tilt; `None` when the device reports none.
    #[must_use]
    pub const fn tilt(&self) -> Option<PointerTilt> {
        self.tilt
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
