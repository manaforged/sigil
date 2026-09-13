#![allow(clippy::redundant_pub_crate)]
#[cfg(test)]
use allocation_counter as _;
#[cfg(test)]
use criterion as _;
mod builder_ext;
mod focus;
mod handlers;
mod hit_test;
mod ime_handlers;
mod pointer_dispatch;
pub mod prelude;
mod router;
mod window_command;

pub use builder_ext::NodeEventsExt;
pub use focus::{FocusIntent, focus_from_keyboard, note_focus_source, request_focus};
pub use handlers::EventHandlers;
pub use router::EventRouter;
pub use window_command::{request_window_command, take_window_commands};

pub fn invoke_node(
    cx: &crate::kernel::Cx,
    layout: &crate::layout::LayoutEngine,
    target: crate::kernel::NodeId,
) {
    pointer_dispatch::invoke_node(cx, layout, target);
}
