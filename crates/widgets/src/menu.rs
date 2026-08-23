//! GPU menus: popup [`menu`] and strip [`menu_bar`].
//!
//! Native OS bars live on the desktop host: `sigil::menu::set_menu_bar`.

mod bar;
mod builder;

pub use bar::{MenuBarBuilder, menu_bar};
pub use builder::{MenuBuilder, MenuItem, menu};
