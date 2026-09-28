use crate::hal::{LogicalSize, ModifiersState, Point};

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum PointerKind {
    #[default]
    Mouse,
    Touch,
    Pen,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Back,
    Forward,
    Other(u16),
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct MouseButtons(u8);

impl MouseButtons {
    pub const EMPTY: Self = Self(0);
    pub const LEFT: Self = Self(1 << 0);
    pub const RIGHT: Self = Self(1 << 1);
    pub const MIDDLE: Self = Self(1 << 2);
    pub const BACK: Self = Self(1 << 3);
    pub const FORWARD: Self = Self(1 << 4);

    #[must_use]
    pub const fn empty() -> Self {
        Self::EMPTY
    }

    #[must_use]
    pub const fn contains(self, button: Self) -> bool {
        self.0 & button.0 == button.0
    }

    pub const fn insert(&mut self, button: Self) {
        self.0 |= button.0;
    }

    pub const fn remove(&mut self, button: Self) {
        self.0 &= !button.0;
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum PointerPhase {
    Down,
    Move,
    Up,
    Enter,
    Leave,
    Cancel,
}

#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct PointerEvent {
    pub position: Point,
    pub window_position: Point,
    pub delta: Point,
    pub kind: PointerKind,
    pub button: Option<MouseButton>,
    pub buttons: MouseButtons,
    pub modifiers: ModifiersState,
    pub phase: PointerPhase,
    pub target_size: LogicalSize,
    capture_requested: bool,
    captured: bool,
    propagation_stopped: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PointerEventInit {
    pub position: Point,
    pub delta: Point,
    pub kind: PointerKind,
    pub button: Option<MouseButton>,
    pub buttons: MouseButtons,
    pub modifiers: ModifiersState,
    pub phase: PointerPhase,
}

impl PointerEvent {
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
            target_size: LogicalSize::new(0.0, 0.0),
            capture_requested: false,
            captured: false,
            propagation_stopped: false,
        }
    }

    pub const fn stop_propagation(&mut self) {
        self.propagation_stopped = true;
    }

    pub const fn capture_pointer(&mut self) {
        self.capture_requested = true;
    }

    #[must_use]
    pub const fn is_captured(&self) -> bool {
        self.captured
    }

    #[must_use]
    pub const fn propagation_stopped(&self) -> bool {
        self.propagation_stopped
    }

    #[must_use]
    pub const fn capture_requested(&self) -> bool {
        self.capture_requested
    }

    pub const fn set_captured(&mut self, captured: bool) {
        self.captured = captured;
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClickConfig {
    pub interval: core::time::Duration,
    pub distance: f32,
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
    pub fn current() -> Self {
        CLICK.with(core::cell::Cell::get).unwrap_or_default()
    }

    pub fn set(config: Self) {
        CLICK.with(|cell| cell.set(Some(config)));
    }

    #[must_use]
    pub fn is_repeat(&self, elapsed: core::time::Duration, from: Point, to: Point) -> bool {
        elapsed <= self.interval
            && (from.x - to.x).abs() <= self.distance
            && (from.y - to.y).abs() <= self.distance
    }
}
