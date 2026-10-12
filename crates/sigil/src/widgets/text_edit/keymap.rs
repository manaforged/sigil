use std::rc::Rc;

use crate::events::{Key, KeyEvent, ModifiersState, NodeEventsExt, Shortcut};
use crate::kernel::NodeBuilder;

use super::super::TextEdit;
use super::navigation::move_horizontal;
use super::{TextEditRuntime, edit_value, insert_text};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::widgets) enum LineStep {
    Up,
    Down,
    PageUp,
    PageDown,
}

const LINE_KEYS: [(Key, LineStep); 4] = [
    (Key::ArrowUp, LineStep::Up),
    (Key::ArrowDown, LineStep::Down),
    (Key::PageUp, LineStep::PageUp),
    (Key::PageDown, LineStep::PageDown),
];

impl LineStep {
    pub(in crate::widgets) fn rows(self, viewport_h: f32, line_h: f32) -> isize {
        #[allow(
            clippy::cast_possible_truncation,
            reason = "row counts are bounded by the viewport height"
        )]
        let page = if viewport_h > 0.0 && line_h > 0.0 {
            ((viewport_h / line_h).floor() as isize).max(1)
        } else {
            10
        };
        match self {
            Self::Up => -1,
            Self::Down => 1,
            Self::PageUp => -page,
            Self::PageDown => page,
        }
    }
}

pub(in crate::widgets) type EdgeKey = Rc<dyn Fn(&mut KeyEvent, bool, bool)>;
pub(in crate::widgets) type LineKey = Rc<dyn Fn(&mut KeyEvent, LineStep, bool)>;

pub(in crate::widgets) struct KeyProfile {
    pub(in crate::widgets) edge: EdgeKey,
    pub(in crate::widgets) line: Option<LineKey>,
}

pub(in crate::widgets) fn edit_keys(
    mut node: NodeBuilder,
    runtime: &TextEditRuntime,
    profile: &KeyProfile,
) -> NodeBuilder {
    let any_key = runtime.clone();
    node = node.key_any(move |event| insert_text(event, &any_key));
    let erase: fn(&mut TextEdit) = TextEdit::backspace;
    for (key, edit) in [
        (Key::Backspace, erase),
        (Key::Delete, TextEdit::delete_forward),
    ] {
        let runtime = runtime.clone();
        node = node.key(key, move |event| edit_value(event, &runtime, edit));
    }
    for extend in [false, true] {
        let shortcut = |key: Key| -> Shortcut {
            if extend {
                Shortcut::new(key, ModifiersState::SHIFT)
            } else {
                key.into()
            }
        };
        for (key, right) in [(Key::ArrowLeft, false), (Key::ArrowRight, true)] {
            let runtime = runtime.clone();
            node = node.key(shortcut(key), move |event| {
                move_horizontal(event, &runtime, right, extend);
            });
        }
        for (key, end) in [(Key::Home, false), (Key::End, true)] {
            let edge = Rc::clone(&profile.edge);
            node = node.key(shortcut(key), move |event| edge(event, end, extend));
        }
        if let Some(line) = &profile.line {
            for (key, step) in LINE_KEYS {
                let line = Rc::clone(line);
                node = node.key(shortcut(key), move |event| line(event, step, extend));
            }
        }
    }
    node
}
