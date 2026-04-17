//! Convenient single-import surface for event users.

pub use crate::{EventHandlers, EventRouter, NodeEventsExt};
pub use sigil_hal::{
    ImeEvent, Key, KeyEvent, KeyPhase, LogicalKey, ModifiersState, MouseButton, MouseButtons,
    PhysicalKey, PointerEvent, PointerKind, PointerPhase, ScrollDelta, ScrollEvent, ScrollPhase,
    ShellEvent, Shortcut, WindowEvent,
};
