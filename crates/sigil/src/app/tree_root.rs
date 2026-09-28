use crate::kernel::{Cx, ImageBuilder, IntoNode, NodeBuilder, NodeId, TextBuilder};
use crate::layout::{Dimension, LayoutStyle};

pub(crate) mod sealed {
    use crate::app::Host;
    use crate::kernel::{Cx, NodeId};

    pub trait Sealed {
        fn mount_into(self, host: &mut Host)
        where
            Self: Sized + super::TreeRoot,
        {
            host.mount(self);
        }
    }

    #[cfg(not(all(feature = "hot", not(target_arch = "wasm32"))))]
    impl<F: FnOnce(&mut Cx) -> NodeId> Sealed for F {}

    #[cfg(all(feature = "hot", not(target_arch = "wasm32")))]
    impl<F: Fn(&mut Cx) -> NodeId + 'static> Sealed for F {
        fn mount_into(self, host: &mut Host) {
            host.mount_hot(self);
        }
    }

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

#[cfg(not(all(feature = "hot", not(target_arch = "wasm32"))))]
impl<F> TreeRoot for F
where
    F: FnOnce(&mut Cx) -> NodeId,
{
    fn install(self, cx: &mut Cx) -> NodeId {
        self(cx)
    }
}

#[cfg(all(feature = "hot", not(target_arch = "wasm32")))]
impl<F> TreeRoot for F
where
    F: Fn(&mut Cx) -> NodeId + 'static,
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

pub(crate) fn fill_root(cx: &mut Cx, root: NodeId) {
    if let Some(node) = cx.get_mut(root) {
        let mut style = node
            .get_extension::<LayoutStyle>()
            .cloned()
            .unwrap_or_default();
        let size = &mut style.data_mut().size;
        if size.width == Dimension::Auto {
            size.width = Dimension::Fill;
        }
        if size.height == Dimension::Auto {
            size.height = Dimension::Fill;
        }
        let _ = node.attach(style);
    }
}

impl TreeRoot for ImageBuilder {
    fn install(self, cx: &mut Cx) -> NodeId {
        cx.add(self)
    }
}
