use sigil::kernel::{Cx, IntoNode, NodeId};

pub struct WithCx<F>(pub F);

impl<F> IntoNode for WithCx<F>
where
    F: FnOnce(&mut Cx) -> NodeId,
{
    fn build(self, cx: &mut Cx) -> NodeId {
        (self.0)(cx)
    }
}
