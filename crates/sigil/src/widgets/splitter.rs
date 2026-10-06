mod atoms;
mod builder;
mod drag;
mod keys;
mod pane;

pub(in crate::widgets) use atoms::{cached_width_atom, to_i16};
pub(in crate::widgets) use builder::HANDLE_PX;
pub use builder::{PaneBuilder, SplitterBuilder, splitter};
pub(in crate::widgets) use keys::{KEY_STEP, resize_keys};
