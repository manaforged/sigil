use crate::kernel::{Cx, NodeId};
use crate::layout::{Dimension, LayoutStyle};

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
