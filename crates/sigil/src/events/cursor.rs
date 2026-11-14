use crate::events::CursorIcon;
use crate::kernel::{Node, Role};
use crate::reactive::{Reactive, Signal};

pub(crate) enum NodeCursor {
    Fixed(CursorIcon),
    Live(Signal<CursorIcon>),
}

impl NodeCursor {
    pub(crate) fn new(cursor: impl Reactive<CursorIcon>) -> Self {
        cursor
            .try_static()
            .map_or_else(|| Self::Live(cursor.into_signal()), Self::Fixed)
    }

    fn get(&self) -> Option<CursorIcon> {
        match self {
            Self::Fixed(cursor) => Some(*cursor),
            Self::Live(signal) => signal.try_peek(),
        }
    }
}

pub(crate) fn node_cursor(node: &Node) -> Option<CursorIcon> {
    if let Some(cursor) = node.extension::<NodeCursor>().and_then(NodeCursor::get) {
        return Some(cursor);
    }
    if node.a11y.is_disabled() {
        return None;
    }
    match node.a11y.role {
        Role::Link => Some(CursorIcon::Pointer),
        role if role.is_text_entry() => Some(CursorIcon::Text),
        _ => None,
    }
}
