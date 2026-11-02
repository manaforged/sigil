use crate::kernel::{Node, NodeBuilder, container};
use crate::widgets::Widget;

use crate::layout::LayoutStyle;
use crate::style::{Metrics, StyleBinding};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Asymmetric(pub &'static str);

/// A container that places its children left to right with the theme gap: 8 px in a window, one cell in a terminal.
#[must_use]
pub fn row() -> NodeBuilder {
    container().layout(LayoutStyle::flex_row().gap(default_gap()))
}

/// A container that places its children top to bottom with the theme gap.
#[must_use]
pub fn column() -> NodeBuilder {
    container().layout(LayoutStyle::flex_column().gap(default_gap()))
}

fn default_gap() -> f32 {
    Metrics::current().gap(crate::style::ControlSize::Default)
}

pub(crate) fn asymmetric(node: NodeBuilder, reason: &'static str) -> NodeBuilder {
    node.mutate(move |node| {
        let _previous = node.attach(Asymmetric(reason));
    })
}

pub(crate) fn install_layout(node: &mut Node, style: &LayoutStyle) {
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
