use crate::kernel::{NodeBuilder, TextBuilder};

use crate::layout::LayoutStyle;

pub trait LayoutBuilderExt {
    #[must_use]
    fn layout(self, style: LayoutStyle) -> Self;
}

impl LayoutBuilderExt for NodeBuilder {
    fn layout(self, style: LayoutStyle) -> Self {
        self.with(style)
    }
}

impl LayoutBuilderExt for TextBuilder {
    fn layout(self, style: LayoutStyle) -> Self {
        self.mutate(move |node| {
            let _ = node.attach(style);
        })
    }
}
