use crate::events::Cursor;
use crate::kernel::{Node, Role};
use crate::reactive::{Reactive, Signal};

pub(crate) enum NodeCursor {
    Fixed(Cursor),
    Live(Signal<Cursor>),
}

impl NodeCursor {
    pub(crate) fn new(cursor: impl Reactive<Cursor>) -> Self {
        match cursor.try_static() {
            Some(cursor) => Self::Fixed(cursor),
            None => Self::Live(cursor.into_signal()),
        }
    }

    fn get(&self) -> Option<Cursor> {
        match self {
            Self::Fixed(cursor) => Some(*cursor),
            Self::Live(signal) => signal.try_peek(),
        }
    }
}

pub(crate) fn node_cursor(node: &Node) -> Option<Cursor> {
    if let Some(cursor) = node.extension::<NodeCursor>().and_then(NodeCursor::get) {
        return Some(cursor);
    }
    if node.a11y.is_disabled() {
        return None;
    }
    match node.a11y.role {
        Role::Link => Some(Cursor::Pointer),
        role if edits_text(role) => Some(Cursor::Text),
        _ => None,
    }
}

const fn edits_text(role: Role) -> bool {
    matches!(
        role,
        Role::TextInput
            | Role::MultilineTextInput
            | Role::SearchInput
            | Role::EmailInput
            | Role::NumberInput
            | Role::PasswordInput
            | Role::PhoneNumberInput
            | Role::UrlInput
    )
}
