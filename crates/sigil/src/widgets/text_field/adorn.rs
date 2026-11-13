use std::cell::Cell;
use std::rc::Rc;

use crate::layout::{AlignItems, LayoutStyle, Space};
use crate::node::{IntoNode, NodeBuilder, NodeId, container};
use crate::reactive::memo;
use crate::style::{StringKey, Theme};
use crate::widgets::icon::DISMISS_GLYPH;
use crate::widgets::{Widget, button};

use super::handlers::TextFieldRuntime;

#[derive(Default)]
pub(super) struct Adornments {
    pub(super) leading: Option<Box<dyn IntoNode>>,
    pub(super) trailing: Option<Box<dyn IntoNode>>,
    pub(super) clear: Option<NodeBuilder>,
}

pub(super) type FocusSlot = Rc<Cell<Option<NodeId>>>;

pub(super) fn clear_button(runtime: &TextFieldRuntime, target: FocusSlot) -> NodeBuilder {
    let value = runtime.state.value();
    let rt = runtime.clone();
    let button = button(DISMISS_GLYPH)
        .ghost()
        .square()
        .activate(move || {
            rt.clear_text();
            if let Some(id) = target.get() {
                id.focus();
            }
        })
        .accessible(Theme::label(StringKey::Clear));
    container()
        .layout(
            LayoutStyle::flex_row()
                .shrink(0.0)
                .align_items(AlignItems::Center),
        )
        .visible(memo(move || !value.get().is_empty()))
        .child(button)
}

impl Adornments {
    pub(super) fn wrap(self, mut root: NodeBuilder, inner: NodeBuilder) -> NodeBuilder {
        if let Some(leading) = self.leading {
            root = root.child(side_slot(leading, true));
        }
        root = root.child(inner);
        if let Some(clear) = self.clear {
            root = root.child(clear);
        }
        if let Some(trailing) = self.trailing {
            root = root.child(side_slot(trailing, false));
        }
        root
    }
}

fn side_slot(child: Box<dyn IntoNode>, leading: bool) -> NodeBuilder {
    let layout = LayoutStyle::flex_row()
        .shrink(0.0)
        .align_items(AlignItems::Center);
    let layout = if leading {
        layout.padding_left(Space::Md)
    } else {
        layout.padding_right(Space::Md)
    };
    container().layout(layout).child(child)
}
