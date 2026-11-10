#![allow(clippy::redundant_pub_crate)]
#[cfg(test)]
use allocation_counter as _;
#[cfg(test)]
use criterion as _;
pub(crate) mod builder_ext;
pub(crate) mod focus;
mod handlers;
pub(crate) mod hit_test;
mod ime_handlers;
pub(crate) mod links;
pub(crate) mod pointer_dispatch;
mod router;
pub(crate) mod selection;
mod selection_walk;
#[cfg(feature = "desktop")]
mod window_command;

pub(crate) use builder_ext::{event_methods, handler_methods, mutate_handlers, with_handlers};
pub(crate) use focus::ScrollReveal;
pub(crate) use focus::focus_from_keyboard;
pub(crate) use focus::note_focus_source;
pub use focus::{FocusDirection, request_focus};
pub(crate) use handlers::EventHandlers;
pub(crate) use handlers::{A11yEdit, dispatch};
#[cfg(not(feature = "kittest"))]
pub(crate) use router::EventRouter;
#[cfg(feature = "kittest")]
#[doc(hidden)]
pub use router::EventRouter;

#[cfg(feature = "desktop")]
pub use window_command::request_window_command;
#[cfg(feature = "desktop")]
pub(crate) use window_command::take_window_commands;

#[cfg(any(test, feature = "kittest"))]
#[doc(hidden)]
pub fn invoke_node(
    cx: &crate::kernel::Cx,
    layout: &crate::layout::LayoutEngine,
    target: crate::kernel::NodeId,
) {
    pointer_dispatch::invoke_node(cx, layout, target, false);
}

pub(crate) use crate::hal::events::ClickConfig;
pub use crate::hal::events::{
    Event, FileDropPhase, Gesture, GestureEvent, ImeEvent, Key, KeyEvent, KeyPhase, LogicalKey,
    ModifiersState, MouseButton, MouseButtons, PhysicalKey, PointerEvent, PointerPhase,
    PointerType, ScrollDelta, ScrollEvent, ScrollPhase, Shortcut, WindowEvent,
};
kittest_pub! {
    #[doc(hidden)]
    pub use crate::hal::events::FileDropEvent;
}
