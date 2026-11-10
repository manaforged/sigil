use crate::kernel::{Cx, NodeId};
use crate::layout::{LayoutStyle, Length};

pub(crate) fn fill_root(cx: &mut Cx, root: NodeId) {
    if let Some(node) = cx.get_mut(root) {
        let mut style = node
            .get_extension::<LayoutStyle>()
            .cloned()
            .unwrap_or_default();
        let size = &mut style.data_mut().size;
        if size.width == Length::Auto {
            size.width = Length::Fill;
        }
        if size.height == Length::Auto {
            size.height = Length::Fill;
        }
        let _ = node.attach(style);
    }
}
