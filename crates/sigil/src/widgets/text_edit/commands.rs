use std::iter::once;

use crate::events::{A11yEdit, with_handlers};
use crate::events::{
    Key, KeyEvent, LogicalKey, ModifiersState, NodeEventsExt, PasteEvent, Shortcut,
};
use crate::kernel::NodeBuilder;

use super::super::TextEdit;
use super::{TextEditRuntime, edit_value, move_caret};

type Command = fn(&mut KeyEvent, &TextEditRuntime);
type Edit = fn(&mut TextEdit);

pub(in crate::widgets) fn clipboard_keys(
    node: NodeBuilder,
    runtime: &TextEditRuntime,
) -> NodeBuilder {
    let pasted = runtime.clone();
    let node = node.on_paste(move |event| shell_paste(event, &pasted));
    let commands: [(&str, ModifiersState, Command); 4] = [
        ("c", ModifiersState::empty(), clipboard_copy),
        ("x", ModifiersState::empty(), clipboard_cut),
        ("v", ModifiersState::empty(), clipboard_paste),
        ("a", ModifiersState::empty(), |event, edit| {
            move_caret(event, edit, TextEdit::select_all);
        }),
    ];
    command_keys(node, runtime, &commands)
}

pub(in crate::widgets) fn history_keys(node: NodeBuilder, runtime: TextEditRuntime) -> NodeBuilder {
    let commands: [(&str, ModifiersState, Command); 2] = [
        ("Z", ModifiersState::SHIFT, |event, edit| {
            history(event, edit, TextEdit::redo);
        }),
        ("z", ModifiersState::empty(), |event, edit| {
            history(event, edit, TextEdit::undo);
        }),
    ];
    command_keys(node, &runtime, &commands).key(
        Shortcut::ctrl(LogicalKey::character("y")),
        move |event| {
            history(event, &runtime, TextEdit::redo);
        },
    )
}

fn command_keys(
    mut node: NodeBuilder,
    runtime: &TextEditRuntime,
    commands: &[(&str, ModifiersState, Command)],
) -> NodeBuilder {
    for modifiers in command_modifiers() {
        for &(key, extra, command) in commands {
            let runtime = runtime.clone();
            let shortcut = Shortcut::new(LogicalKey::character(key), modifiers | extra);
            node = node.key(shortcut, move |event| command(event, &runtime));
        }
    }
    node
}

fn command_modifiers() -> impl Iterator<Item = ModifiersState> {
    once(ModifiersState::CONTROL).chain(cfg!(target_os = "macos").then_some(ModifiersState::META))
}

pub(in crate::widgets) fn word_keys(
    mut node: NodeBuilder,
    runtime: &TextEditRuntime,
) -> NodeBuilder {
    let alt = |key: LogicalKey| Shortcut::new(key, ModifiersState::ALT);
    let ch = |key: &str| LogicalKey::character(key);
    let motions: [(Shortcut, Edit); 4] = [
        (alt(Key::ArrowLeft.into()), TextEdit::move_word_left),
        (alt(Key::ArrowRight.into()), TextEdit::move_word_right),
        (alt(ch("b")), TextEdit::move_word_left),
        (alt(ch("f")), TextEdit::move_word_right),
    ];
    for (shortcut, edit) in motions {
        let runtime = runtime.clone();
        node = node.key(shortcut, move |event| move_caret(event, &runtime, edit));
    }
    let deletions: [(Shortcut, Edit); 5] = [
        (alt(Key::Backspace.into()), TextEdit::delete_word_backward),
        (Shortcut::ctrl(ch("w")), TextEdit::delete_word_backward),
        (alt(ch("d")), TextEdit::delete_word_forward),
        (Shortcut::ctrl(ch("u")), TextEdit::delete_to_line_start),
        (Shortcut::ctrl(ch("k")), TextEdit::delete_to_line_end),
    ];
    for (shortcut, edit) in deletions {
        let runtime = runtime.clone();
        node = node.key(shortcut, move |event| edit_value(event, &runtime, edit));
    }
    node
}

pub(in crate::widgets) fn submit_text(event: &mut KeyEvent, runtime: &TextEditRuntime) {
    event.stop_propagation();
    if runtime.disabled() {
        return;
    }
    let value = runtime.value.peek();
    (runtime.submit)(value.as_str());
}

fn clipboard_copy(event: &mut KeyEvent, runtime: &TextEditRuntime) {
    event.stop_propagation();
    if runtime.disabled() {
        return;
    }
    let state = runtime.edit_mut();
    if let Some(text) = state.selected_text() {
        runtime
            .clip_fail
            .set(crate::clipboard::write(&*runtime.clipboard, text).is_err());
    }
}

fn clipboard_cut(event: &mut KeyEvent, runtime: &TextEditRuntime) {
    event.stop_propagation();
    if runtime.disabled() {
        return;
    }
    let selected = runtime.edit_mut().selected_text().map(ToOwned::to_owned);
    if let Some(text) = selected {
        let failed = crate::clipboard::write(&*runtime.clipboard, &text).is_err();
        runtime.clip_fail.set(failed);
        if failed {
            return;
        }
        let mut state = runtime.edit_mut();
        state.backspace();
        runtime.sync_value(&state);
    }
}

fn clipboard_paste(event: &mut KeyEvent, runtime: &TextEditRuntime) {
    if runtime.disabled() {
        event.stop_propagation();
        return;
    }
    let Some(text) = runtime.clipboard.get() else {
        return;
    };
    event.stop_propagation();
    runtime.insert_pasted(&text);
}

fn shell_paste(event: &mut PasteEvent, runtime: &TextEditRuntime) {
    event.stop_propagation();
    if !runtime.disabled() {
        runtime.insert_pasted(&event.text);
    }
}

fn history(event: &mut KeyEvent, runtime: &TextEditRuntime, step: fn(&mut TextEdit) -> bool) {
    event.stop_propagation();
    if runtime.disabled() {
        return;
    }
    let mut state = runtime.edit_mut();
    if step(&mut state) {
        runtime.sync_value(&state);
    }
}

pub(in crate::widgets) fn attach_a11y_edit(
    node: NodeBuilder,
    runtime: TextEditRuntime,
) -> NodeBuilder {
    with_handlers(node, move |handlers| {
        handlers.set_a11y_edit(Box::new(move |edit| runtime.a11y_edit(edit)));
    })
}

impl TextEditRuntime {
    fn insert_pasted(&self, text: &str) {
        let mut state = self.edit_mut();
        let normalised = if self.column.is_some() {
            text.chars()
                .filter(|ch| *ch != '\n' && *ch != '\r')
                .collect()
        } else {
            text.replace("\r\n", "\n").replace('\r', "\n")
        };
        state.insert_str(&normalised);
        self.sync_value(&state);
    }

    fn a11y_edit(&self, edit: &A11yEdit) -> bool {
        if self.disabled() {
            return true;
        }
        let mut state = self.edit_mut();
        match edit {
            A11yEdit::Text(text) => {
                if self.column.is_some() {
                    let line: String = text.chars().filter(|ch| !ch.is_control()).collect();
                    state.replace(&line);
                } else {
                    state.replace(text);
                }
                self.sync_value(&state);
            }
            A11yEdit::Select { anchor, focus } => {
                state.place_caret(*anchor);
                state.extend_to(*focus);
                self.commit(&state);
            }
            A11yEdit::Number(_) => return false,
        }
        true
    }
}
