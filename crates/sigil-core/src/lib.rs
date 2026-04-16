//! Core node tree, component composition, and semantics.
//!
//! `sigil-core` is the keystone crate for Sigil. It owns
//! stable node identity, the arena-backed tree model, the open
//! [`Component`] trait, accessible-by-default [`Semantics`], and the
//! hand-written builders used by higher layers.

// `pub(crate)` on module seams documents the intended boundary even
// when clippy's nursery lint sees a currently-private path.
#![allow(clippy::redundant_pub_crate)]

#[cfg(test)]
use criterion as _;

mod builder;
mod component;
mod cx;
mod id;
mod node;
pub mod prelude;
mod semantics;
mod text;

pub use accesskit::{Action, Role};
pub use builder::{NodeBuilder, container, fragment};
pub use component::Component;
pub use cx::Cx;
pub use id::NodeId;
pub use node::{Node, NodeExtensions, NodeKind};
pub use semantics::{NameSource, Semantics};
pub use text::{TextBuilder, TextSource, text};
