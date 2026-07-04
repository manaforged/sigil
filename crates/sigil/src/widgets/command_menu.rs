//! Command menu widget — the FFT-style nested action menu (Attack /
//! Ability > sub-list / Item / Wait), keyboard-driven and cancellable.
//!
//! Up/Down move within the highlighted level (skipping disabled rows).
//! Enter or Right descends into a submenu, or fires a leaf and closes
//! back to the root. Esc or Left ascends one level, or fires
//! [`CommandMenuBuilder::cancel`] at the root level. See
//! [`command_menu`] for the authoring surface.

mod builder;
mod nav;
mod render;

pub use builder::{CommandMenuBuilder, SubmenuBuilder, command_menu};
