//! Component trait and object-erased child storage.

use crate::{Cx, NodeId};

/// One-shot construction of a node subtree.
///
/// Components are consumed when built. State lives in signals and in
/// the node tree, not in a retained component instance.
pub trait Component {
    /// Build this component and return the root node ID it created.
    fn build(self, cx: &mut Cx) -> NodeId;
}

impl<F> Component for F
where
    F: FnOnce(&mut Cx) -> NodeId,
{
    fn build(self, cx: &mut Cx) -> NodeId {
        self(cx)
    }
}

pub(crate) trait BoxedComponent {
    fn build_box(self: Box<Self>, cx: &mut Cx) -> NodeId;
}

impl<T> BoxedComponent for T
where
    T: Component,
{
    fn build_box(self: Box<Self>, cx: &mut Cx) -> NodeId {
        (*self).build(cx)
    }
}
