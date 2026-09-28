use std::iter::once;

use crate::events::prelude::{Key, KeyEvent, LogicalKey, ModifiersState, NodeEventsExt, Shortcut};
use crate::events::{A11yEdit, with_handlers};
use crate::kernel::NodeBuilder;

use super::super::TextEdit;
use super::{TextEditRuntime, commit, edit_value, move_caret, sync_value};

pub(in crate::widgets) fn clipboard_keys(
    mut node: NodeBuilder,
    runtime: &TextEditRuntime,
) -> NodeBuilder {
    for modifiers in once(ModifiersState::CONTROL)
        .chain(cfg!(target_os = "macos").then_some(ModifiersState::META))
    {
        let copy = runtime.clone();
        let cut = runtime.clone();
        let paste = runtime.clone();
        let select_all = runtime.clone();
        node = node
            .key(
                Shortcut::new(LogicalKey::character("c"), modifiers),
                move |event| clipboard_copy(event, &copy),
            )
            .key(
                Shortcut::new(LogicalKey::character("x"), modifiers),
                move |event| clipboard_cut(event, &cut),
            )
            .key(
                Shortcut::new(LogicalKey::character("v"), modifiers),
                move |event| clipboard_paste(event, &paste),
            )
            .key(
                Shortcut::new(LogicalKey::character("a"), modifiers),
                move |event| select_all_text(event, &select_all),
            );
    }
    node
}

pub(in crate::widgets) fn history_keys(
    mut node: NodeBuilder,
    runtime: TextEditRuntime,
) -> NodeBuilder {
    for modifiers in once(ModifiersState::CONTROL)
        .chain(cfg!(target_os = "macos").then_some(ModifiersState::META))
    {
        let redo = runtime.clone();
        let undo = runtime.clone();
        node = node
            .key(
                Shortcut::new(
                    LogicalKey::character("Z"),
                    modifiers | ModifiersState::SHIFT,
                ),
                move |event| history_redo(event, &redo),
            )
            .key(
                Shortcut::new(LogicalKey::character("z"), modifiers),
                move |event| history_undo(event, &undo),
            );
    }
    node.key(Shortcut::ctrl(LogicalKey::character("y")), move |event| {
        history_redo(event, &runtime);
    })
}

type Edit = fn(&mut TextEdit);

pub(in crate::widgets) fn word_keys(node: NodeBuilder, runtime: &TextEditRuntime) -> NodeBuilder {
    let node = motion_keys(node, runtime);
    deletion_keys(node, runtime)
}

fn motion_keys(mut node: NodeBuilder, runtime: &TextEditRuntime) -> NodeBuilder {
    let bindings: [(Shortcut, Edit); 4] = [
        (
            Shortcut::new(Key::ArrowLeft, ModifiersState::ALT),
            TextEdit::move_word_left,
        ),
        (
            Shortcut::new(Key::ArrowRight, ModifiersState::ALT),
            TextEdit::move_word_right,
        ),
        (
            Shortcut::new(LogicalKey::character("b"), ModifiersState::ALT),
            TextEdit::move_word_left,
        ),
        (
            Shortcut::new(LogicalKey::character("f"), ModifiersState::ALT),
            TextEdit::move_word_right,
        ),
    ];
    for (shortcut, edit) in bindings {
        let runtime = runtime.clone();
        node = node.key(shortcut, move |event| move_caret(event, &runtime, edit));
    }
    node
}

fn deletion_keys(mut node: NodeBuilder, runtime: &TextEditRuntime) -> NodeBuilder {
    let bindings: [(Shortcut, Edit); 5] = [
        (
            Shortcut::new(Key::Backspace, ModifiersState::ALT),
            TextEdit::delete_word_backward,
        ),
        (
            Shortcut::ctrl(LogicalKey::character("w")),
            TextEdit::delete_word_backward,
        ),
        (
            Shortcut::new(LogicalKey::character("d"), ModifiersState::ALT),
            TextEdit::delete_word_forward,
        ),
        (
            Shortcut::ctrl(LogicalKey::character("u")),
            TextEdit::delete_to_line_start,
        ),
        (
            Shortcut::ctrl(LogicalKey::character("k")),
            TextEdit::delete_to_line_end,
        ),
    ];
    for (shortcut, edit) in bindings {
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
        sync_value(runtime, &state);
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
    let mut state = runtime.edit_mut();
    let normalised = if runtime.column.is_some() {
        text.chars()
            .filter(|ch| *ch != '\n' && *ch != '\r')
            .collect()
    } else {
        text.replace("\r\n", "\n").replace('\r', "\n")
    };
    state.insert_str(&normalised);
    sync_value(runtime, &state);
}

fn select_all_text(event: &mut KeyEvent, runtime: &TextEditRuntime) {
    event.stop_propagation();
    if !runtime.disabled() {
        let mut state = runtime.edit_mut();
        state.select_all();
        commit(runtime, &state);
    }
}

fn history_undo(event: &mut KeyEvent, runtime: &TextEditRuntime) {
    event.stop_propagation();
    if runtime.disabled() {
        return;
    }
    let mut state = runtime.edit_mut();
    if state.undo() {
        sync_value(runtime, &state);
    }
}

fn history_redo(event: &mut KeyEvent, runtime: &TextEditRuntime) {
    event.stop_propagation();
    if runtime.disabled() {
        return;
    }
    let mut state = runtime.edit_mut();
    if state.redo() {
        sync_value(runtime, &state);
    }
}

pub(in crate::widgets) fn attach_a11y_edit(
    node: NodeBuilder,
    runtime: TextEditRuntime,
) -> NodeBuilder {
    with_handlers(node, move |handlers| {
        handlers.set_a11y_edit(Box::new(move |edit| a11y_edit(&runtime, edit)));
    })
}

fn a11y_edit(runtime: &TextEditRuntime, edit: &A11yEdit) -> bool {
    if runtime.disabled() {
        return true;
    }
    let mut state = runtime.edit_mut();
    match edit {
        A11yEdit::Text(text) if runtime.column.is_some() => {
            let line: String = text.chars().filter(|ch| !ch.is_control()).collect();
            state.replace(&line);
            sync_value(runtime, &state);
        }
        A11yEdit::Text(text) => {
            state.replace(text);
            sync_value(runtime, &state);
        }
        A11yEdit::Select { anchor, focus } => {
            state.place_caret(*anchor);
            state.extend_to(*focus);
            commit(runtime, &state);
        }
        A11yEdit::Number(_) => return false,
    }
    true
}
