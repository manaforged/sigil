use crate::layout::Space;
use std::cell::Cell;
use std::rc::Rc;

use crate::kernel::{IntoNode, NodeBuilder, NodeId, container};
use crate::style::strings::StringKey;
use crate::style::theme::ambient::theme_label;
use crate::widgets::button::button;
use crate::widgets::icon::DISMISS_GLYPH;
use crate::widgets::{Layout, Widget};

use super::handlers::TextFieldRuntime;
use crate::layout::{AlignItems, LayoutStyle};

#[derive(Default)]
pub(super) struct Adornments {
    pub(super) leading: Option<Box<dyn IntoNode>>,
    pub(super) trailing: Option<Box<dyn IntoNode>>,
    pub(super) clear: Option<NodeBuilder>,
}

pub(super) type FocusSlot = Rc<Cell<Option<NodeId>>>;

pub(super) fn clear_button(runtime: &TextFieldRuntime, target: FocusSlot) -> NodeBuilder {
    let value = runtime.input.value;
    let input = runtime.input.clone();
    let button = button(DISMISS_GLYPH)
        .ghost()
        .square()
        .activate(move || {
            if input.disabled() {
                return;
            }
            let mut state = input.edit_mut();
            state.replace("");
            input.sync_value(&state);
            drop(state);
            if let Some(id) = target.get() {
                crate::events::request_focus(id);
            }
        })
        .accessible(theme_label(StringKey::Clear));
    container()
        .layout(
            LayoutStyle::flex_row()
                .shrink(0.0)
                .align_items(AlignItems::Center),
        )
        .visible_when(crate::reactive::memo(move || !value.get().is_empty()))
        .child(button)
}

impl Adornments {
    pub(super) fn wrap(self, node: NodeBuilder, inner: NodeBuilder) -> NodeBuilder {
        let node = match self.leading {
            Some(leading) => node.child(slot(leading, true)),
            None => node,
        };
        let node = node.child(inner);
        let node = match self.clear {
            Some(clear) => node.child(clear),
            None => node,
        };
        match self.trailing {
            Some(trailing) => node.child(slot(trailing, false)),
            None => node,
        }
    }
}

fn slot(content: Box<dyn IntoNode>, leading: bool) -> NodeBuilder {
    let layout = LayoutStyle::flex_row()
        .shrink(0.0)
        .align_items(AlignItems::Center);
    let layout = if leading {
        layout.padding_left(Space::Md)
    } else {
        layout.padding_right(Space::Md)
    };
    container().layout(layout).child(content)
}
