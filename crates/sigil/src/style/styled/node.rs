use crate::kernel::NodeBuilder;
use crate::style::{ClassSource, Style, StyleOverlay};

pub(crate) fn attach_style(builder: NodeBuilder, mut style: Style) -> NodeBuilder {
    if style.is_empty() {
        return crate::widgets::wire(builder, StyleOverlay::default());
    }
    let images = style.take_images();
    let builder = if images.is_empty() {
        builder
    } else {
        builder.mutate(move |node| images.install(node))
    };
    let builder = crate::widgets::wire(builder, style.take_state_overlay());
    let base_atoms = style.into_atoms();
    if base_atoms.is_empty() {
        return builder;
    }
    builder.class(ClassSource::Owned(Box::new(base_atoms)))
}
