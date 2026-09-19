use std::iter::once;

use crate::events::prelude::{KeyEvent, LogicalKey, ModifiersState, NodeEventsExt, Shortcut};
use crate::kernel::NodeBuilder;

use super::{TextEditRuntime, commit, sync_value};

pub(super) fn clipboard_keys(mut node: NodeBuilder, runtime: &TextEditRuntime) -> NodeBuilder {
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

pub(super) fn history_keys(mut node: NodeBuilder, runtime: TextEditRuntime) -> NodeBuilder {
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

pub(super) fn submit_text(event: &mut KeyEvent, runtime: &TextEditRuntime) {
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
    let state = runtime.edit.borrow();
    if let Some(text) = state.selected_text() {
        drop(crate::widgets::clipboard::write(&*runtime.clipboard, text));
    }
}

fn clipboard_cut(event: &mut KeyEvent, runtime: &TextEditRuntime) {
    event.stop_propagation();
    if runtime.disabled() {
        return;
    }
    let selected = runtime.edit.borrow().selected_text().map(ToOwned::to_owned);
    if let Some(text) = selected {
        if crate::widgets::clipboard::write(&*runtime.clipboard, &text).is_err() {
            return;
        }
        let mut state = runtime.edit_mut();
        state.backspace();
        sync_value(runtime, &state);
    }
}

fn clipboard_paste(event: &mut KeyEvent, runtime: &TextEditRuntime) {
    event.stop_propagation();
    if runtime.disabled() {
        return;
    }
    let Some(text) = runtime.clipboard.get() else {
        return;
    };
    let mut state = runtime.edit_mut();
    let normalised = text.replace("\r\n", "\n").replace('\r', "\n");
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
