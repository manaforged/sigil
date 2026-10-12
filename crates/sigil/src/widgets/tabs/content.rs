use crate::kernel::{IntoNode, NodeBuilder, Role, container};
use crate::layout::{AlignItems, LayoutBuilderExt, LayoutStyle};
use crate::reactive::Signal;
use crate::style::{DisplayKind, StyleAtom, space};

pub(super) fn wrap_with_content(
    tab_list: NodeBuilder,
    active: Signal<usize>,
    contents: Vec<Box<dyn IntoNode>>,
) -> NodeBuilder {
    let root = container()
        .role(Role::GenericContainer)
        .layout(
            LayoutStyle::flex_column()
                .grow(1.0)
                .align_items(AlignItems::Stretch)
                .gap(space(2)),
        )
        .child(tab_list);
    contents
        .into_iter()
        .enumerate()
        .fold(root, |root, (index, content)| {
            root.child(
                container()
                    .role(Role::GenericContainer)
                    .layout(LayoutStyle::flex_column().grow(1.0))
                    .style_reactive(move || -> &'static [StyleAtom] {
                        if active.get() == index {
                            &[]
                        } else {
                            &[StyleAtom::Display(DisplayKind::None)]
                        }
                    })
                    .child(content),
            )
        })
}
