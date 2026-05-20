//! Menu widget — floating vertical stack of menu items with
//! optional shortcut hints, separators, submenu indicators, and a
//! reactive `open` signal that gates visibility.

mod builder;

pub use builder::{MenuBuilder, MenuItem, menu};
