use crate::{Cx, NodeId};

pub trait IntoNode: IntoNodeBox {
    fn build(self, cx: &mut Cx) -> NodeId;
}

pub trait IntoNodeBox {
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

#[must_use]
pub fn node(child: impl IntoNode + 'static) -> Box<dyn IntoNode> {
    Box::new(child)
}

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
