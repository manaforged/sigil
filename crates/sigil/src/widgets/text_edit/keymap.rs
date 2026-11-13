use std::rc::Rc;

use crate::events::{Key, KeyEvent, Modifiers, NamedKey, Shortcut};
use crate::kernel::NodeBuilder;
use crate::widgets::Widget;

use super::super::TextEdit;
use super::navigation::move_horizontal;
use super::{TextEditRuntime, edit_value, insert_text};

const FALLBACK_PAGE: isize = 10;

pub(in crate::widgets) type Edit = fn(&mut TextEdit);

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

const KEY_ACTIONS: [(NamedKey, KeyAction); 10] = [
    (NamedKey::Backspace, KeyAction::Edit(EditOp::Backspace)),
    (NamedKey::Delete, KeyAction::Edit(EditOp::Delete)),
    (NamedKey::ArrowLeft, KeyAction::Move(Motion::Left)),
    (NamedKey::ArrowRight, KeyAction::Move(Motion::Right)),
    (NamedKey::Home, KeyAction::Move(Motion::Home)),
    (NamedKey::End, KeyAction::Move(Motion::End)),
    (NamedKey::ArrowUp, KeyAction::Line(LineStep::Up)),
    (NamedKey::ArrowDown, KeyAction::Line(LineStep::Down)),
    (NamedKey::PageUp, KeyAction::Line(LineStep::PageUp)),
    (NamedKey::PageDown, KeyAction::Line(LineStep::PageDown)),
];

impl EditOp {
    fn edit(self) -> Edit {
        match self {
            Self::Backspace => TextEdit::backspace,
            Self::Delete => TextEdit::delete_forward,
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

pub(in crate::widgets) fn delete_keys() -> impl Iterator<Item = (Shortcut, Edit)> {
    let alt = |key: Key| Shortcut::new(key, Modifiers::ALT);
    let ch = |key: &str| Key::character(key);
    let words: [(Shortcut, Edit); 5] = [
        (
            alt(NamedKey::Backspace.into()),
            TextEdit::delete_word_backward,
        ),
        (Shortcut::ctrl(ch("w")), TextEdit::delete_word_backward),
        (alt(ch("d")), TextEdit::delete_word_forward),
        (Shortcut::ctrl(ch("u")), TextEdit::delete_to_line_start),
        (Shortcut::ctrl(ch("k")), TextEdit::delete_to_line_end),
    ];
    KEY_ACTIONS
        .into_iter()
        .filter_map(|(key, action)| match action {
            KeyAction::Edit(op) => Some((key, op.edit())),
            KeyAction::Move(_) | KeyAction::Line(_) => None,
        })
        .flat_map(|(key, edit)| {
            [
                (Shortcut::from(key), edit),
                (Shortcut::new(key, Modifiers::SHIFT), edit),
            ]
        })
        .chain(words)
}

pub(in crate::widgets) fn edit_keys(
    mut node: NodeBuilder,
    runtime: &TextEditRuntime,
    profile: &KeyProfile,
) -> NodeBuilder {
    let any_key = runtime.clone();
    node = node.key_any(move |event| insert_text(event, &any_key));
    for (shortcut, edit) in delete_keys() {
        let runtime = runtime.clone();
        node = node.key(shortcut, move |event| edit_value(event, &runtime, edit));
    }
    for (key, action) in KEY_ACTIONS {
        if matches!(action, KeyAction::Edit(_)) {
            continue;
        }
        for extend in [false, true] {
            let shortcut = if extend {
                Shortcut::new(key, Modifiers::SHIFT)
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
