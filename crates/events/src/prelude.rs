//! Convenient single-import surface for event users.

pub use crate::{EventHandlers, EventRouter, FocusIntent, NodeEventsExt};
pub use hal::{
    ImeEvent, Key, KeyEvent, KeyPhase, LogicalKey, ModifiersState, MouseButton, MouseButtons,
    PhysicalKey, PointerEvent, PointerKind, PointerPhase, ScrollDelta, ScrollEvent, ScrollPhase,
    ShellEvent, Shortcut, WindowEvent,
};
