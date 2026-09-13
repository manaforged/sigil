use crate::kernel::{Cx, ImageBuilder, IntoNode, NodeBuilder, NodeId, TextBuilder};

mod sealed {
    pub trait Sealed {}
    impl<F: FnOnce(&mut crate::kernel::Cx) -> crate::kernel::NodeId> Sealed for F {}
    impl Sealed for crate::kernel::NodeBuilder {}
    impl Sealed for crate::kernel::TextBuilder {}
    impl Sealed for crate::kernel::ImageBuilder {}
    impl<N: crate::kernel::IntoNode> Sealed for super::Root<N> {}
}

#[must_use]
pub fn root(node: impl IntoNode) -> impl TreeRoot {
    Root(node)
}

struct Root<N: IntoNode>(N);

impl<N: IntoNode> TreeRoot for Root<N> {
    fn install(self, cx: &mut Cx) -> NodeId {
        cx.add(self.0)
    }
}

pub trait TreeRoot: sealed::Sealed {
    fn install(self, cx: &mut Cx) -> NodeId;
}

impl<F> TreeRoot for F
where
    F: FnOnce(&mut Cx) -> NodeId,
{
    fn install(self, cx: &mut Cx) -> NodeId {
        self(cx)
    }
}

impl TreeRoot for NodeBuilder {
    fn install(self, cx: &mut Cx) -> NodeId {
        cx.add(self)
    }
}

impl TreeRoot for TextBuilder {
    fn install(self, cx: &mut Cx) -> NodeId {
        cx.add(self)
    }
}

impl TreeRoot for ImageBuilder {
    fn install(self, cx: &mut Cx) -> NodeId {
        cx.add(self)
    }
}
