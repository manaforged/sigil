use sigil_kernel::{IntoNode, NodeBuilder, Role, container};
use sigil_layout::prelude::{AlignItems, LayoutBuilderExt, LayoutStyle};
use sigil_reactive::Signal;
use sigil_style::{DisplayKind, NodeStyleExt, StyleAtom};

use super::builder::{ContentFactory, ContentSlot};

pub(super) fn wrap_with_content(
    tab_list: NodeBuilder,
    active: Signal<usize>,
    contents: Vec<Option<ContentSlot>>,
    count: usize,
) -> NodeBuilder {
    let mut root = container()
        .role(Role::GenericContainer)
        .layout(LayoutStyle::flex_column().align_items(AlignItems::Stretch))
        .child(tab_list);

    let mut factories: Vec<Option<ContentFactory>> = (0..count).map(|_| None).collect();
    let mut any_factory = false;

    for (index, slot) in contents.into_iter().enumerate() {
        match slot {
            Some(ContentSlot::Static(node)) => {
                root = root.child(content_panel(active, index, node, count));
            }
            Some(ContentSlot::Factory(factory)) => {
                if let Some(slot_mut) = factories.get_mut(index) {
                    *slot_mut = Some(factory);
                    any_factory = true;
                }
            }
            None => {}
        }
    }

    if any_factory {
        root = root.child(reactive_content(active, factories));
    }
    root
}

fn reactive_content(active: Signal<usize>, factories: Vec<Option<ContentFactory>>) -> NodeBuilder {
    container()
        .role(Role::GenericContainer)
        .layout(LayoutStyle::flex_column().align_items(AlignItems::Stretch))
        .children_reactive(move || {
            let idx = active.get();
            factories
                .get(idx)
                .and_then(|slot| slot.as_ref().map(|factory| factory()))
                .into_iter()
                .collect::<Vec<_>>()
        })
}

fn content_panel(
    active: Signal<usize>,
    index: usize,
    node: Box<dyn IntoNode>,
    _count: usize,
) -> NodeBuilder {
    container()
        .role(Role::GenericContainer)
        .layout(LayoutStyle::flex_column())
        .style_reactive(move || content_display(active, index))
        .child(node)
}

fn content_display(active: Signal<usize>, index: usize) -> &'static [StyleAtom] {
    if active.get() == index {
        &[]
    } else {
        &[StyleAtom::Display(DisplayKind::None)]
    }
}
