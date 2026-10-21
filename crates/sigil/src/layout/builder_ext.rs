use crate::kernel::{Node, NodeBuilder, TextBuilder, container};

use crate::layout::LayoutStyle;
use crate::layout::align::Asymmetric;
use crate::style::{Metrics, StyleBinding};

#[must_use]
pub fn row() -> NodeBuilder {
    container().layout(LayoutStyle::flex_row().gap(default_gap()))
}

#[must_use]
pub fn column() -> NodeBuilder {
    container().layout(LayoutStyle::flex_column().gap(default_gap()))
}

fn default_gap() -> f32 {
    Metrics::current().gap(crate::style::ControlSize::Default)
}

pub trait LayoutBuilderExt {
    #[must_use]
    fn layout(self, style: LayoutStyle) -> Self;

    #[must_use]
    fn asymmetric(self, reason: &'static str) -> Self;
}

impl LayoutBuilderExt for NodeBuilder {
    fn layout(self, style: LayoutStyle) -> Self {
        self.mutate(move |node| install_layout(node, &style))
    }

    fn asymmetric(self, reason: &'static str) -> Self {
        self.mutate(move |node| {
            let _previous = node.attach(Asymmetric(reason));
        })
    }
}

impl LayoutBuilderExt for TextBuilder {
    fn layout(self, style: LayoutStyle) -> Self {
        self.mutate(move |node| install_layout(node, &style))
    }

    fn asymmetric(self, reason: &'static str) -> Self {
        self.mutate(move |node| {
            let _previous = node.attach(Asymmetric(reason));
        })
    }
}

fn install_layout(node: &mut Node, style: &LayoutStyle) {
    let slots = style.slots();
    if let Some(binding) = node.get_extension_mut::<StyleBinding>() {
        binding.strip(slots);
        binding.note_user(slots);
    }
    if let Some(existing) = node.get_extension_mut::<LayoutStyle>() {
        existing.merge(style);
    } else {
        let _previous = node.attach(style.clone());
    }
}
