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
pub use image::{ImageBuilder, ImageCrop, ImageSource, ObjectFit, image};
pub use node::{Node, NodeExtensions, NodeType, Surface, TopLayer};
pub use reactive_children::schedule_keyed_rebuild;
pub use semantics::{A11yProps, AccessibleName, FocusScopeKind};
pub use text::{TextBuilder, TextContent, text, text_live};
