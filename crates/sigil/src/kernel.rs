#![allow(clippy::redundant_pub_crate)]

#[cfg(test)]
use criterion as _;

macro_rules! shared_node_methods {
    (@mutate $(, $field:ident)*) => {
        /// Runs `apply` on the node when it is built, before its children are added.
        pub fn mutate(mut self, apply: impl FnOnce(&mut $crate::kernel::Node) + 'static) -> Self {
            self$(.$field)*.pre_extensions.push(Box::new(apply));
            self
        }
    };
    ($($field:ident),*) => {
        /// Sets the accessibility role of the node.
        pub const fn role(mut self, role: $crate::kernel::Role) -> Self {
            self$(.$field)*.a11y.role = role;
            self
        }

        /// Sets the accessible name of the node.
        pub fn accessible(mut self, name: impl Into<$crate::kernel::AccessibleName>) -> Self {
            self$(.$field)*.a11y.name = name.into();
            self
        }

        /// Sets the accessible description of the node.
        pub fn description(mut self, description: impl Into<String>) -> Self {
            self$(.$field)*.a11y.description =
                Some($crate::kernel::AccessibleName::Static(description.into()));
            self
        }

        /// Sets whether the node and its subtree stay out of the accessibility tree, like `aria-hidden`.
        pub fn accessibility_hidden(mut self, yes: bool) -> Self {
            if self$(.$field)*.a11y.hidden != yes {
                self$(.$field)*.a11y.hidden = yes;
            }
            self
        }

        /// Sets whether the node can take keyboard focus; a fragment cannot.
        pub fn focusable(mut self, yes: bool) -> Self {
            debug_assert!(
                !(yes && matches!(self$(.$field)*.kind, $crate::kernel::NodeType::Fragment)),
                "focusable(true) on a Fragment: fragments have no geometry"
            );
            self$(.$field)*.a11y.focusable = yes;
            self
        }

        /// Sets the tab order: lower indexes come first, and a negative index skips the node.
        pub fn tab_index(mut self, index: i32) -> Self {
            self$(.$field)*.a11y.tab_index = Some(index);
            self
        }

        /// Sets whether pointer hit tests and focus moves can land on the node.
        pub const fn hittable(mut self, yes: bool) -> Self {
            self$(.$field)*.a11y.hittable = yes;
            self
        }

        shared_node_methods!(@mutate $(, $field)*);

        /// Adds this node to the tree and returns its id.
        #[allow(
            clippy::should_implement_trait,
            reason = "intentional, reviewed trade-off"
        )]
        pub fn add(self, cx: &mut $crate::kernel::Cx) -> $crate::kernel::NodeId {
            cx.add(self)
        }
    };
}

mod access;
mod announce;
mod builder;
mod builder_wrap;
mod component;
mod control_flow;
mod cx;
mod delta;
mod describe;
pub(crate) mod hooks;
mod id;
mod image;
mod image_cache;
#[cfg(feature = "image-decode")]
mod image_load;
mod node;
mod reactive_children;
mod semantics;
mod text;
mod top_layer;

pub use access::{Action, Live, Role, SortDirection};
pub use announce::announce;
pub use builder::{NodeBuilder, container, fragment};
#[doc(hidden)]
pub use component::NodePatch;
#[cfg(not(feature = "kittest"))]
pub(crate) use component::WithCx;
#[cfg(feature = "kittest")]
#[doc(hidden)]
pub use component::WithCx;
pub(crate) use component::into_node_builder;
pub use component::{IntoNode, node};
pub use control_flow::{SwitchBuilder, for_each, keyed, show, switch};
pub use cx::{Cx, HostMode};
pub use delta::{DeltaKind, FrameDelta, NodeTag};
pub use id::NodeId;
pub(crate) use image::ThemedImage;
pub use image::{ImageBuilder, ImageCrop, ImageRaster, ImageSource, ObjectFit, image};
pub(crate) use image_cache::cached_image;
#[cfg(feature = "image-decode")]
pub(crate) use image_load::Load;
pub use node::{Node, NodeExtensions, Surface, TopLayer};
kittest_pub! {
    #[doc(hidden)]
    pub use node::NodeType;
}
#[cfg(feature = "kittest")]
pub use reactive_children::schedule_keyed_rebuild;
pub(crate) use reactive_children::schedule_keyed_rebuild_at;
pub(crate) use semantics::TextCursor;
pub use semantics::{A11yDetail, A11yProps, AccessibleName, FocusScopeKind};
pub use text::{TextBuilder, TextContent, rich, text};
