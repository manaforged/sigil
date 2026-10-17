#![allow(clippy::redundant_pub_crate)]

#[cfg(test)]
use criterion as _;

mod announce;
mod builder;
mod builder_wrap;
mod component;
mod control_flow;
mod cx;
mod delta;
mod describe;
mod id;
mod image;
mod image_cache;
mod node;
mod reactive_children;
mod semantics;
mod text;
mod top_layer;

pub use accesskit::{Action, Live, Role, SortDirection};
pub(crate) use announce::queue_announcement;
pub use announce::{announce, announce_assertive};
pub use builder::{NodeBuilder, container, fragment};
pub(crate) use component::WithCx;
pub use component::{IntoNode, node};
pub use control_flow::{StatusView, SwitchBuilder, for_each, match_status, show, switch};
pub use cx::{Cx, HostMode};
pub use delta::{DeltaKind, FrameDelta, NodeTag};
pub use id::NodeId;
pub(crate) use image::ThemedImage;
pub use image::{ImageBuilder, ImageCrop, ImageRaster, ImageSource, ObjectFit, image};
pub(crate) use image_cache::cached_image;
pub use node::{Node, NodeExtensions, NodeType, Surface, TopLayer};
pub use reactive_children::schedule_keyed_rebuild;
pub use semantics::{A11yProps, AccessibleName, FocusScopeKind, TextCursor};
pub use text::{TextBuilder, TextContent, text};
