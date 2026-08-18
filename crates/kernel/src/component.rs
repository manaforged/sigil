//! `IntoNode` trait and object-erased child storage.

use crate::{Cx, NodeId};

/// Anything that can produce a node in the tree when built.
///
/// Implement this to write a custom component. The simplest form is a
/// plain closure — `|cx: &mut Cx| { ... }` — but you can also implement
/// the trait on a struct to bundle configuration:
///
/// ```ignore
/// struct Button { label: &'static str }
///
/// impl IntoNode for Button {
///     fn build(self, cx: &mut Cx) -> NodeId {
///         cx.add(container().role(Role::Button).label(self.label))
///     }
/// }
/// ```
///
/// All state lives in signals and in the node tree, not in the `IntoNode`
/// value — it is consumed on build and does not persist.
pub trait IntoNode: IntoNodeBox {
    /// Build this value into the tree and return the root [`NodeId`] it created.
    fn build(self, cx: &mut Cx) -> NodeId;
}

/// Object-safe boxed build entry point for [`IntoNode`] trait objects.
pub trait IntoNodeBox {
    /// Build this boxed value into the tree and return the root [`NodeId`] it created.
    fn build_box(self: Box<Self>, cx: &mut Cx) -> NodeId;
}

impl<T> IntoNodeBox for T
where
    T: IntoNode,
{
    fn build_box(self: Box<Self>, cx: &mut Cx) -> NodeId {
        (*self).build(cx)
    }
}

impl IntoNode for Box<dyn IntoNode> {
    fn build(self, cx: &mut Cx) -> NodeId {
        <dyn IntoNode as IntoNodeBox>::build_box(self, cx)
    }
}

impl<F> IntoNode for F
where
    F: FnOnce(&mut Cx) -> NodeId,
{
    fn build(self, cx: &mut Cx) -> NodeId {
        self(cx)
    }
}

/// Type-erase a child so mixed widgets can sit in one list.
///
/// `children_reactive` and `children_keyed` need one item type. Wrap
/// each child with `node(...)` when the list mixes `text`, `chip`,
/// builders, and closures.
#[must_use]
pub fn node(child: impl IntoNode + 'static) -> Box<dyn IntoNode> {
    Box::new(child)
}

/// Object-safe wrapper that lets any [`IntoNode`] be stored as `Box<dyn ErasedComponent>`.
///
/// This exists because `IntoNode::build(self, …)` takes `self` by value, which is not
/// object-safe. `ErasedComponent::build_box` takes `Box<Self>` instead, allowing
/// heterogeneous children to be collected in a single `SmallVec` without an enum.
pub(crate) trait ErasedComponent {
    fn build_box(self: Box<Self>, cx: &mut Cx) -> NodeId;
}

impl<T> ErasedComponent for T
where
    T: IntoNode,
{
    fn build_box(self: Box<Self>, cx: &mut Cx) -> NodeId {
        IntoNodeBox::build_box(self, cx)
    }
}
