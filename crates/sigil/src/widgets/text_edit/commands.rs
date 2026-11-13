use std::iter::once;

use crate::events::{A11yEdit, with_handlers};
use crate::events::{Key, KeyEvent, Modifiers, NamedKey, Shortcut};
use crate::kernel::NodeBuilder;
use crate::widgets::Widget;

use super::super::TextEdit;
use super::{TextEditRuntime, edit_value, move_caret};

type KeyCommand = fn(&mut KeyEvent, &TextEditRuntime);
pub(in crate::widgets) type KeyBinding<R> = (&'static str, Modifiers, fn(&mut KeyEvent, &R));
type Edit = fn(&mut TextEdit);

pub(in crate::widgets) fn clipboard_keys(
    node: NodeBuilder,
    runtime: &TextEditRuntime,
) -> NodeBuilder {
    let pasted = runtime.clone();
    let node = node.paste(move |text| shell_paste(text, &pasted));
    let commands: [(&str, Modifiers, KeyCommand); 4] = [
        ("c", Modifiers::empty(), clipboard_copy),
        ("x", Modifiers::empty(), clipboard_cut),
        ("v", Modifiers::empty(), clipboard_paste),
        ("a", Modifiers::empty(), |event, edit| {
            move_caret(event, edit, TextEdit::select_all);
        }),
    ];
    bind_command_keys(node, runtime, &commands)
}

pub(in crate::widgets) fn history_keys(node: NodeBuilder, runtime: TextEditRuntime) -> NodeBuilder {
    bind_history_keys(
        node,
        runtime,
        |event, edit| history(event, edit, TextEdit::undo),
        |event, edit| history(event, edit, TextEdit::redo),
    )
}

const TERMINAL_UNDO: [&str; 3] = ["_", "-", "7"];

pub(in crate::widgets) fn bind_command_keys<R: Clone + 'static>(
    mut node: NodeBuilder,
    runtime: &R,
    commands: &[KeyBinding<R>],
) -> NodeBuilder {
    for modifiers in command_modifiers() {
        for &(key, extra, command) in commands {
            let runtime = runtime.clone();
            let shortcut = Shortcut::new(Key::character(key), modifiers | extra);
            node = node.key(shortcut, move |event| command(event, &runtime));
        }
    }
    node
}

pub(in crate::widgets) fn bind_history_keys<R: Clone + 'static>(
    node: NodeBuilder,
    runtime: R,
    undo: fn(&mut KeyEvent, &R),
    redo: fn(&mut KeyEvent, &R),
) -> NodeBuilder {
    let commands: [KeyBinding<R>; 2] = [
        ("Z", Modifiers::SHIFT, redo),
        ("z", Modifiers::empty(), undo),
    ];
    let node = bind_command_keys(node, &runtime, &commands);
    let node = TERMINAL_UNDO.iter().fold(node, |node, key| {
        let rt = runtime.clone();
        node.key(Shortcut::ctrl(Key::character(*key)), move |event| {
            undo(event, &rt);
        })
    });
    node.key(Shortcut::ctrl(Key::character("y")), move |event| {
        redo(event, &runtime);
    })
}

fn command_modifiers() -> impl Iterator<Item = Modifiers> {
    once(Modifiers::CTRL).chain(cfg!(target_os = "macos").then_some(Modifiers::META))
}

pub(in crate::widgets) fn word_keys(
    mut node: NodeBuilder,
    runtime: &TextEditRuntime,
) -> NodeBuilder {
    let alt = |key: Key| Shortcut::new(key, Modifiers::ALT);
    let ch = |key: &str| Key::character(key);
    let motions: [(Shortcut, Edit); 4] = [
        (alt(NamedKey::ArrowLeft.into()), TextEdit::move_word_left),
        (alt(NamedKey::ArrowRight.into()), TextEdit::move_word_right),
        (alt(ch("b")), TextEdit::move_word_left),
        (alt(ch("f")), TextEdit::move_word_right),
    ];
    for (shortcut, edit) in motions {
        let runtime = runtime.clone();
        node = node.key(shortcut, move |event| move_caret(event, &runtime, edit));
    }
    let deletions: [(Shortcut, Edit); 5] = [
        (
            alt(NamedKey::Backspace.into()),
            TextEdit::delete_word_backward,
        ),
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
    if runtime.disabled() {
        event.stop_propagation();
        return;
    }
    let state = runtime.edit_mut();
    if let Some(text) = state.selected_text() {
        event.prevent_default();
        event.stop_propagation();
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

fn shell_paste(text: &str, runtime: &TextEditRuntime) -> bool {
    if !runtime.disabled() {
        runtime.insert_pasted(text);
    }
    true
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
        let normalised = text.replace("\r\n", "\n").replace('\r', "\n");
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
                state.replace(text);
                self.sync_value(&state);
            }
            A11yEdit::Select { anchor, focus } => {
                state.place_caret(*anchor);
                state.extend_to(*focus);
                self.commit(&state);
            }
            A11yEdit::Increment | A11yEdit::Decrement | _ => return false,
        }
        true
    }
}
