use std::cell::Cell;
use std::rc::Rc;

use crate::kernel::{IntoNode, NodeBuilder, NodeId, container};
use crate::style::NodeStyleExt;
use crate::widgets::button::icon_button;
use crate::widgets::text_edit::sync_value;

use super::handlers::TextFieldRuntime;
use crate::layout::{AlignItems, LayoutBuilderExt, LayoutStyle};
use crate::style::space;

#[derive(Default)]
pub(super) struct Adornments {
    pub(super) leading: Option<Box<dyn IntoNode>>,
    pub(super) trailing: Option<Box<dyn IntoNode>>,
    pub(super) clear: Option<NodeBuilder>,
}

pub(super) type FocusTarget = Rc<Cell<Option<NodeId>>>;

const CLEAR_GLYPH: &str = "\u{d7}";
const CLEAR_LABEL: &str = "Clear";

pub(super) fn clear_button(runtime: &TextFieldRuntime, target: FocusTarget) -> NodeBuilder {
    let value = runtime.input.value;
    let input = runtime.input.clone();
    let button = icon_button(CLEAR_GLYPH, move || {
        if input.disabled() {
            return;
        }
        let mut state = input.edit_mut();
        state.replace("");
        sync_value(&input, &state);
        drop(state);
        if let Some(id) = target.get() {
            crate::events::request_focus(id);
        }
    })
    .accessible(CLEAR_LABEL);
    container()
        .layout(
            LayoutStyle::flex_row()
                .shrink(0.0)
                .align_items(AlignItems::Center),
        )
        .visible_when(crate::reactive::derived(move || !value.get().is_empty()))
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
        layout.padding_left(space(3))
    } else {
        layout.padding_right(space(3))
    };
    container().layout(layout).child(content)
}
