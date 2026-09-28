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
pub mod prelude;
mod router;
pub(crate) mod selection;
mod selection_walk;
mod window_command;

pub use builder_ext::NodeEventsExt;
pub(crate) use builder_ext::{mutate_handlers, with_handlers};
pub(crate) use focus::ScrollReveal;
pub use focus::{
    FocusIntent, focus_from_keyboard, note_focus_source, request_focus, request_reveal,
};
pub use handlers::EventHandlers;
pub(crate) use handlers::{A11yEdit, dispatch};
pub use router::EventRouter;
pub use selection::SelectableText;
pub use window_command::{request_window_command, take_window_commands};

#[cfg(any(test, feature = "kittest"))]
pub fn invoke_node(
    cx: &crate::kernel::Cx,
    layout: &crate::layout::LayoutEngine,
    target: crate::kernel::NodeId,
) {
    pointer_dispatch::invoke_node(cx, layout, target);
}
