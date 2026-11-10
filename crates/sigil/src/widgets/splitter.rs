mod builder;
mod drag;
mod keys;
mod pane;
mod pane_builder;

pub(in crate::widgets) use crate::widgets::scroll::to_i16;
pub(in crate::widgets) use builder::HANDLE_PX;
pub use builder::{SplitterBuilder, splitter};
pub(in crate::widgets) use keys::{KEY_STEP, resize_keys};
pub(in crate::widgets) use pane::cached_width_atom;
pub use pane_builder::PaneBuilder;
