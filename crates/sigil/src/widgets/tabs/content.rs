use crate::kernel::{IntoNode, NodeBuilder, Role, container};
use crate::layout::Display;
use crate::layout::Space;
use crate::layout::{AlignItems, LayoutStyle};
use crate::reactive::Signal;
use crate::style::StyleAtom;
use crate::widgets::Layout;

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
                .min_size(0.0, 0.0)
                .align_items(AlignItems::Stretch)
                .gap(Space::Sm),
        )
        .child(tab_list);
    contents
        .into_iter()
        .enumerate()
        .fold(root, |root, (index, content)| {
            root.child(
                container()
                    .role(Role::GenericContainer)
                    .layout(LayoutStyle::flex_column().grow(1.0).min_size(0.0, 0.0))
                    .style_reactive(move || -> &'static [StyleAtom] {
                        if active.get() == index {
                            &[]
                        } else {
                            &[StyleAtom::Display(Display::None)]
                        }
                    })
                    .child(content),
            )
        })
}
