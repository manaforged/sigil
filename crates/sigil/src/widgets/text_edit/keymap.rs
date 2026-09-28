use std::rc::Rc;

use crate::events::prelude::{Key, KeyEvent, ModifiersState, NodeEventsExt, Shortcut};
use crate::kernel::NodeBuilder;

use super::super::TextEdit;
use super::navigation::move_horizontal;
use super::{TextEditRuntime, edit_value, insert_text};

const FALLBACK_PAGE: isize = 10;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::widgets) enum EditOp {
    Backspace,
    Delete,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::widgets) enum Motion {
    Left,
    Right,
    Home,
    End,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::widgets) enum LineStep {
    Up,
    Down,
    PageUp,
    PageDown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::widgets) enum KeyAction {
    Edit(EditOp),
    Move(Motion),
    Line(LineStep),
}

const KEY_ACTIONS: [(Key, KeyAction); 10] = [
    (Key::Backspace, KeyAction::Edit(EditOp::Backspace)),
    (Key::Delete, KeyAction::Edit(EditOp::Delete)),
    (Key::ArrowLeft, KeyAction::Move(Motion::Left)),
    (Key::ArrowRight, KeyAction::Move(Motion::Right)),
    (Key::Home, KeyAction::Move(Motion::Home)),
    (Key::End, KeyAction::Move(Motion::End)),
    (Key::ArrowUp, KeyAction::Line(LineStep::Up)),
    (Key::ArrowDown, KeyAction::Line(LineStep::Down)),
    (Key::PageUp, KeyAction::Line(LineStep::PageUp)),
    (Key::PageDown, KeyAction::Line(LineStep::PageDown)),
];

impl EditOp {
    fn apply(self, edit: &mut TextEdit) {
        match self {
            Self::Backspace => edit.backspace(),
            Self::Delete => edit.delete_forward(),
        }
    }
}

impl LineStep {
    pub(in crate::widgets) fn rows(self, viewport_h: f32, line_h: f32) -> isize {
        let page = page_rows(viewport_h, line_h).unwrap_or(FALLBACK_PAGE);
        match self {
            Self::Up => -1,
            Self::Down => 1,
            Self::PageUp => -page,
            Self::PageDown => page,
        }
    }
}

fn page_rows(viewport_h: f32, line_h: f32) -> Option<isize> {
    if !(viewport_h > 0.0 && line_h > 0.0) {
        return None;
    }
    #[allow(
        clippy::cast_possible_truncation,
        reason = "row counts are bounded by the viewport height"
    )]
    let rows = (viewport_h / line_h).floor() as isize;
    Some(rows.max(1))
}

pub(in crate::widgets) type EdgeHandler = Rc<dyn Fn(&mut KeyEvent, bool, bool)>;
pub(in crate::widgets) type LineHandler = Rc<dyn Fn(&mut KeyEvent, LineStep, bool)>;

type MotionHandler = Rc<dyn Fn(&mut KeyEvent)>;

pub(in crate::widgets) struct KeyProfile {
    pub(in crate::widgets) edge: EdgeHandler,
    pub(in crate::widgets) line: Option<LineHandler>,
}

pub(in crate::widgets) fn edit_keys(
    mut node: NodeBuilder,
    runtime: &TextEditRuntime,
    profile: &KeyProfile,
) -> NodeBuilder {
    let any_key = runtime.clone();
    node = node.key_any(move |event| insert_text(event, &any_key));
    for (key, action) in KEY_ACTIONS {
        if let KeyAction::Edit(op) = action {
            let runtime = runtime.clone();
            node = node.key(key, move |event| {
                edit_value(event, &runtime, |edit| op.apply(edit));
            });
            continue;
        }
        for extend in [false, true] {
            let shortcut = if extend {
                Shortcut::new(key, ModifiersState::SHIFT)
            } else {
                key.into()
            };
            if let Some(handler) = motion_handler(action, runtime, profile, extend) {
                node = node.key(shortcut, move |event| handler(event));
            }
        }
    }
    node
}

fn motion_handler(
    action: KeyAction,
    runtime: &TextEditRuntime,
    profile: &KeyProfile,
    extend: bool,
) -> Option<MotionHandler> {
    match action {
        KeyAction::Edit(_) => None,
        KeyAction::Move(Motion::Left | Motion::Right) => {
            let runtime = runtime.clone();
            let right = action == KeyAction::Move(Motion::Right);
            Some(Rc::new(move |event| {
                move_horizontal(event, &runtime, right, extend);
            }))
        }
        KeyAction::Move(motion) => {
            let edge = Rc::clone(&profile.edge);
            let end = motion == Motion::End;
            Some(Rc::new(move |event| edge(event, end, extend)))
        }
        KeyAction::Line(step) => {
            let line = profile.line.clone()?;
            Some(Rc::new(move |event| line(event, step, extend)))
        }
    }
}
