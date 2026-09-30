use crate::kernel::{IntoNode, NodeBuilder, Role, container};
use crate::layout::{AlignItems, LayoutBuilderExt, LayoutStyle};
use crate::reactive::Signal;
use crate::style::{DisplayKind, NodeStyleExt, StyleAtom, space};

pub(super) fn wrap_with_content(
    tab_list: NodeBuilder,
    active: Signal<usize>,
    contents: Vec<Option<Box<dyn IntoNode>>>,
) -> NodeBuilder {
    let root = container()
        .role(Role::GenericContainer)
        .layout(
            LayoutStyle::flex_column()
                .align_items(AlignItems::Stretch)
                .gap(space(2)),
        )
        .child(tab_list);
    contents
        .into_iter()
        .enumerate()
        .filter_map(|(index, content)| content.map(|content| (index, content)))
        .fold(root, |root, (index, content)| {
            root.child(content_panel(active, index, content))
        })
}

fn content_panel(active: Signal<usize>, index: usize, content: Box<dyn IntoNode>) -> NodeBuilder {
    container()
        .role(Role::GenericContainer)
        .layout(LayoutStyle::flex_column())
        .style_reactive(move || content_display(active, index))
        .child(content)
}

fn content_display(active: Signal<usize>, index: usize) -> &'static [StyleAtom] {
    if active.get() == index {
        &[]
    } else {
        &[StyleAtom::Display(DisplayKind::None)]
    }
}
