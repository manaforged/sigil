//! Core node tree, component composition, and semantics.
//!
//! `sigil-kernel` is the keystone crate for Sigil. It owns
//! stable node identity, the arena-backed tree model, the open
//! [`IntoNode`] trait, accessible-by-default [`A11yProps`], and the
//! hand-written builders used by higher layers.
//!
//! # Reactive ownership
//!
//! Every node in the tree is tied to a *reactive owner* — a scope that
//! tracks signal subscriptions and cleans them up when the scope is
//! disposed. [`Cx`] holds one root owner; every call to [`Cx::add`] or
//! [`Cx::add_child`] creates a child owner scoped to that component.
//! When you remove a subtree with [`Cx::remove`], the engine walks the
//! subtree in child-before-parent order, disposes each component's owner,
//! and cancels any reactive subscriptions that were created inside it.
//! You do not need to manage owners manually — the tree drives it.

#![deny(missing_docs)]
// `pub(crate)` on module seams documents the intended boundary even
// when clippy's nursery lint sees a currently-private path.
#![allow(clippy::redundant_pub_crate)]

#[cfg(test)]
use criterion as _;

mod announce;
mod builder;
mod component;
mod cx;
mod id;
mod image;
mod interval;
mod node;
pub mod prelude;
mod reactive_children;
mod semantics;
mod text;

pub use accesskit::{Action, Live, Role, SortDirection};
pub use announce::queue_announcement;
pub use builder::{NodeBuilder, container, fragment};
pub use component::{IntoNode, node};
pub use cx::Cx;
pub use id::NodeId;
pub use image::{ImageBuilder, ImageSource, ObjectFit, image};
pub use node::{Node, NodeExtensions, NodeType, Surface, TopLayer};
pub use reactive_children::schedule_keyed_rebuild;
pub use semantics::{A11yProps, AccessibleName, FocusScopeKind};
pub use text::{TextBuilder, TextContent, text, text_live};
