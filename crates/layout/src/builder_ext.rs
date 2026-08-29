use sigil_kernel::{ImageBuilder, NodeBuilder, TextBuilder};

use crate::LayoutStyle;

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

impl LayoutBuilderExt for ImageBuilder {
    fn layout(self, style: LayoutStyle) -> Self {
        self.mutate(move |node| {
            let _ = node.attach(style);
        })
    }
}
