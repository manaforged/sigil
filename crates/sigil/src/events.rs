#![allow(clippy::redundant_pub_crate)]
#[cfg(test)]
use allocation_counter as _;
#[cfg(test)]
use criterion as _;
mod builder_ext;
pub(crate) mod focus;
mod handlers;
pub(crate) mod hit_test;
mod ime_handlers;
pub(crate) mod pointer_dispatch;
mod router;
pub(crate) mod selection;
mod selection_walk;
#[cfg(feature = "desktop")]
mod window_command;

pub use builder_ext::NodeEventsExt;
pub(crate) use builder_ext::{mutate_handlers, with_handlers};
pub(crate) use focus::ScrollReveal;
pub(crate) use focus::note_focus_source;
pub use focus::{FocusIntent, focus_from_keyboard, request_focus, request_reveal};
pub use handlers::EventHandlers;
pub(crate) use handlers::{A11yEdit, dispatch};
pub use router::EventRouter;
pub use selection::SelectableText;
#[cfg(feature = "desktop")]
pub use window_command::request_window_command;
#[cfg(feature = "desktop")]
pub(crate) use window_command::take_window_commands;

#[cfg(any(test, feature = "kittest"))]
pub fn invoke_node(
    cx: &crate::kernel::Cx,
    layout: &crate::layout::LayoutEngine,
    target: crate::kernel::NodeId,
) {
    pointer_dispatch::invoke_node(cx, layout, target);
}

pub use crate::hal::events::{
    ClickConfig, FileDropEvent, FileDropPhase, ImeEvent, Key, KeyEvent, KeyEventInit, KeyPhase,
    LogicalKey, ModifiersState, MouseButton, MouseButtons, PRIMARY, PhysicalKey, PointerEvent,
    PointerEventInit, PointerKind, PointerPhase, ScrollDelta, ScrollEvent, ScrollEventInit,
    ScrollPhase, ShellEvent, Shortcut, WindowEvent,
};
