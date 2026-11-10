use crate::kernel::{Cx, NodeId};

#[derive(Debug)]
pub enum Patch<T> {
    Patched,
    Mismatch(T),
    Unsupported(T),
}

impl<T> Patch<T> {
    pub(crate) fn map<U>(self, wrap: impl FnOnce(T) -> U) -> Patch<U> {
        match self {
            Self::Patched => Patch::Patched,
            Self::Mismatch(value) => Patch::Mismatch(wrap(value)),
            Self::Unsupported(value) => Patch::Unsupported(wrap(value)),
        }
    }
}

/// A value that builds into a node.
pub trait IntoNode: IntoNodeBox {
    /// Adds this value to the tree and returns its node id.
    fn build(self, cx: &mut Cx) -> NodeId;

    /// Updates the existing node in place when this value allows it.
    #[doc(hidden)]
    fn patch(self, _cx: &mut Cx, _existing: NodeId) -> Patch<Self>
    where
        Self: Sized,
    {
        Patch::Unsupported(self)
    }
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

impl<F, N> IntoNode for F
where
    F: FnOnce() -> N,
    N: IntoNode,
{
    fn build(self, cx: &mut Cx) -> NodeId {
        self().build(cx)
    }
}

#[doc(hidden)]
pub struct WithCx<F>(pub F);

impl<F> core::fmt::Debug for WithCx<F> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("WithCx").finish_non_exhaustive()
    }
}

impl<F> IntoNode for WithCx<F>
where
    F: FnOnce(&mut Cx) -> NodeId,
{
    fn build(self, cx: &mut Cx) -> NodeId {
        (self.0)(cx)
    }
}

/// Box a child of any builder type.
#[must_use]
pub fn node(child: impl IntoNode + 'static) -> Box<dyn IntoNode> {
    Box::new(child)
}

macro_rules! into_node_builder {
    ($($ty:ty),* $(,)?) => {
        $(
            impl From<$ty> for crate::kernel::NodeBuilder {
                fn from(builder: $ty) -> Self {
                    crate::kernel::NodeBuilder::wrapping(builder)
                }
            }
        )*
    };
}

pub(crate) use into_node_builder;

into_node_builder!(crate::kernel::TextBuilder, crate::kernel::ImageBuilder,);

pub(crate) trait ErasedComponent {
    fn build_box(self: Box<Self>, cx: &mut Cx) -> NodeId;

    fn patch_box(self: Box<Self>, cx: &mut Cx, existing: NodeId)
    -> Patch<Box<dyn ErasedComponent>>;
}

impl<T> ErasedComponent for T
where
    T: IntoNode + 'static,
{
    fn build_box(self: Box<Self>, cx: &mut Cx) -> NodeId {
        IntoNodeBox::build_box(self, cx)
    }

    fn patch_box(
        self: Box<Self>,
        cx: &mut Cx,
        existing: NodeId,
    ) -> Patch<Box<dyn ErasedComponent>> {
        (*self)
            .patch(cx, existing)
            .map(|value| Box::new(value) as Box<dyn ErasedComponent>)
    }
}
