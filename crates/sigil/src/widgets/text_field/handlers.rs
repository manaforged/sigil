use std::rc::Rc;

use crate::clipboard::Clipboard;
use crate::events::{A11yEdit, ImeEvent, Key, KeyEvent, KeyPhase, Modifiers, NamedKey, Shortcut};
use crate::geometry::Rect;
use crate::node::NodeBuilder;
use crate::reactive::{Signal, effect, memo};
use crate::text::{CaretGeometry, HitPosition, TextStyle, measure_text};
use crate::widgets::shared::TextChange;
use crate::widgets::text_edit::{
    bind_command_keys, bind_history_keys, compose_caret, typed_event_text,
};
use crate::widgets::{TextState, Widget};

type EditAction = fn(&TextFieldRuntime);
type KeyCommand = (
    &'static str,
    Modifiers,
    fn(&mut KeyEvent, &TextFieldRuntime),
);

#[derive(Clone)]
pub(super) struct TextFieldRuntime {
    pub(super) state: TextState,
    pub(super) disabled: Option<Signal<bool>>,
    pub(super) change: TextChange,
    pub(super) submit: TextChange,
    pub(super) clipboard: Rc<dyn Clipboard>,
    pub(super) caret_visible: Signal<bool>,
    pub(super) text_style: Signal<TextStyle>,
    pub(super) scroll_x: Signal<f32>,
    pub(super) column: f32,
    pub(super) clip_fail: Signal<bool>,
    pub(super) focused: Signal<bool>,
    pub(super) preedit: Signal<String>,
    pub(super) preedit_cursor: Signal<Option<(usize, usize)>>,
    pub(super) caret_rect: Option<Signal<Option<Rect>>>,
    pub(super) password: bool,
    pub(super) reveal: Signal<bool>,
}

impl TextFieldRuntime {
    pub(super) fn disabled(&self) -> bool {
        self.disabled.is_some_and(|flag| flag.peek())
    }

    pub(super) fn hiding(&self) -> bool {
        super::mask::hiding(self.password, self.reveal)
    }

    pub(super) fn hiding_peek(&self) -> bool {
        super::mask::hiding_peek(self.password, self.reveal)
    }

    pub(super) fn clear_text(&self) {
        if self.disabled() {
            return;
        }
        self.state.replace("");
        self.sync_value();
    }

    pub(super) fn commit_caret(&self) {
        self.keep_caret_visible();
    }

    pub(super) fn sync_value(&self) {
        self.keep_caret_visible();
        let value = self.state.value().peek();
        (self.change)(value.as_str());
    }

    pub(super) fn composing_caret(&self, text: &str, caret: usize) -> (String, usize) {
        let preedit = self.preedit.get();
        let cursor = self.preedit_cursor.get();
        compose_caret(text, caret, &preedit, cursor)
    }

    fn keep_caret_visible(&self) {
        self.caret_visible.set(true);
        let value = self.state.value().peek();
        let caret = self.state.caret().peek();
        let preedit = self.preedit.peek();
        let cursor = self.preedit_cursor.peek();
        let (composed, byte) = compose_caret(&value, caret, &preedit, cursor);
        let style = self.text_style.peek();
        let hiding = self.hiding_peek();
        let (layout, mapped) = super::mask::layout_text(&composed, byte, hiding);
        let Some(pos) = measure_text(|engine| {
            engine.caret(&layout, &style, None, HitPosition::downstream(mapped))
        }) else {
            return;
        };
        let mut scroll = self.scroll_x.peek();
        let x = pos.x();
        if x < scroll {
            scroll = x;
        } else if x > scroll + self.column {
            scroll = x - self.column;
        }
        self.scroll_x.set(scroll.max(0.0));
    }

    fn caret_geometry(&self) -> Option<CaretGeometry> {
        if self.disabled.is_some_and(|flag| flag.get()) {
            return None;
        }
        let value = self.state.value().get();
        let caret = self.state.caret().get();
        let (composed, byte) = self.composing_caret(&value, caret);
        let hiding = self.hiding();
        let (layout, mapped) = super::mask::layout_text(&composed, byte, hiding);
        let style = self.text_style.get();
        let pos = measure_text(|engine| {
            engine.caret(&layout, &style, None, HitPosition::downstream(mapped))
        })?;
        let scroll = self.scroll_x.get().max(0.0);
        Some(CaretGeometry::new(
            (pos.x() - scroll).max(0.0),
            pos.top(),
            pos.height(),
        ))
    }

    fn insert_pasted(&self, text: &str) {
        let line: String = text
            .chars()
            .filter(|ch| *ch != '\n' && *ch != '\r')
            .collect();
        self.state.insert(&line);
        self.sync_value();
    }

    fn delete_to_edge(&self, to_end: bool) {
        if self.state.selection().is_some() {
            self.state.clear_selection();
        }
        let caret = self.state.caret().peek();
        if to_end {
            let len = self.state.value().peek().len();
            if caret < len {
                self.state.select(caret);
                self.state.extend(len);
                self.state.backspace();
            }
        } else if caret > 0 {
            self.state.select(0);
            self.state.extend(caret);
            self.state.backspace();
        }
        self.sync_value();
    }

    fn move_edge(&self, end: bool, extend: bool) {
        let to = if end {
            self.state.value().peek().len()
        } else {
            0
        };
        if extend {
            self.state.extend(to);
        } else {
            self.state.select(to);
        }
        self.commit_caret();
    }

    fn clear_preedit(&self) {
        if !self.preedit.peek().is_empty() {
            self.preedit.set(String::new());
        }
        if self.preedit_cursor.peek().is_some() {
            self.preedit_cursor.set(None);
        }
    }

    fn handle_ime(&self, event: &ImeEvent) {
        if matches!(event, ImeEvent::Disabled) {
            self.clear_preedit();
            return;
        }
        if self.disabled() {
            return;
        }
        match event {
            ImeEvent::Preedit(text, range) => {
                if text.is_empty() {
                    self.clear_preedit();
                } else {
                    crate::reactive::batch(|| {
                        self.preedit.set(text.clone());
                        self.preedit_cursor.set(*range);
                    });
                }
            }
            ImeEvent::Commit(text) => {
                self.clear_preedit();
                self.insert_pasted(text);
            }
            ImeEvent::Enabled | ImeEvent::Disabled => {}
        }
    }

    fn handle_a11y_edit(&self, edit: &A11yEdit) -> bool {
        if self.disabled() {
            return true;
        }
        match edit {
            A11yEdit::Text(text) => {
                let line: String = text.chars().filter(|ch| !ch.is_control()).collect();
                self.state.replace(&line);
                self.sync_value();
            }
            A11yEdit::Select { anchor, focus } => {
                self.state.select(*anchor);
                self.state.extend(*focus);
                self.commit_caret();
            }
            _ => return false,
        }
        true
    }
}

pub(super) fn attach_key_handlers(node: NodeBuilder, runtime: &TextFieldRuntime) -> NodeBuilder {
    let node = attach_navigation_keys(node, runtime);
    let node = attach_edit_keys(node, runtime);
    let node = attach_clipboard_keys(node, runtime);
    let node = attach_history_keys(node, runtime);
    let node = attach_text_input(node, runtime);
    attach_ime_and_a11y(node, runtime)
}

fn attach_navigation_keys(mut node: NodeBuilder, runtime: &TextFieldRuntime) -> NodeBuilder {
    for (key, right) in [(NamedKey::ArrowLeft, false), (NamedKey::ArrowRight, true)] {
        for (mods, extend) in [(Modifiers::empty(), false), (Modifiers::SHIFT, true)] {
            let rt = runtime.clone();
            node = node.key(Shortcut::new(key, mods), move |event| {
                run_caret_key(event, &rt, |s| s.move_by(right, extend));
            });
        }
    }
    for (key, end) in [(NamedKey::Home, false), (NamedKey::End, true)] {
        for (mods, extend) in [(Modifiers::empty(), false), (Modifiers::SHIFT, true)] {
            let rt = runtime.clone();
            node = node.key(Shortcut::new(key, mods), move |event| {
                run_caret_action(event, &rt, |r| r.move_edge(end, extend));
            });
        }
    }
    #[cfg(target_os = "macos")]
    for (key, end) in [(NamedKey::ArrowLeft, false), (NamedKey::ArrowRight, true)] {
        for (mods, extend) in [
            (Modifiers::META, false),
            (Modifiers::META | Modifiers::SHIFT, true),
        ] {
            let rt = runtime.clone();
            node = node.key(Shortcut::new(key, mods), move |event| {
                run_caret_action(event, &rt, |r| r.move_edge(end, extend));
            });
        }
    }
    for (shortcut, word_right) in [
        (Shortcut::new(NamedKey::ArrowLeft, Modifiers::ALT), false),
        (Shortcut::new(Key::character("b"), Modifiers::ALT), false),
        (Shortcut::new(NamedKey::ArrowRight, Modifiers::ALT), true),
        (Shortcut::new(Key::character("f"), Modifiers::ALT), true),
    ] {
        let rt = runtime.clone();
        node = node.key(shortcut, move |event| {
            run_caret_key(event, &rt, |s| {
                if word_right {
                    s.move_word_right();
                } else {
                    s.move_word_left();
                }
            });
        });
    }
    node
}

fn attach_edit_keys(mut node: NodeBuilder, runtime: &TextFieldRuntime) -> NodeBuilder {
    let delete_shortcuts: [(Shortcut, EditAction); 9] = [
        (Shortcut::plain(NamedKey::Backspace), |r| {
            r.state.backspace();
            r.sync_value();
        }),
        (Shortcut::new(NamedKey::Backspace, Modifiers::SHIFT), |r| {
            r.state.backspace();
            r.sync_value();
        }),
        (Shortcut::plain(NamedKey::Delete), |r| {
            r.state.delete_forward();
            r.sync_value();
        }),
        (Shortcut::new(NamedKey::Delete, Modifiers::SHIFT), |r| {
            r.state.delete_forward();
            r.sync_value();
        }),
        (Shortcut::new(NamedKey::Backspace, Modifiers::ALT), |r| {
            r.state.delete_word_backward();
            r.sync_value();
        }),
        (Shortcut::ctrl(Key::character("w")), |r| {
            r.state.delete_word_backward();
            r.sync_value();
        }),
        (Shortcut::new(Key::character("d"), Modifiers::ALT), |r| {
            r.state.delete_word_forward();
            r.sync_value();
        }),
        (Shortcut::ctrl(Key::character("u")), |r| {
            r.delete_to_edge(false);
        }),
        (Shortcut::ctrl(Key::character("k")), |r| {
            r.delete_to_edge(true);
        }),
    ];
    for (shortcut, op) in delete_shortcuts {
        let rt = runtime.clone();
        node = node.key(shortcut, move |event| run_edit_action(event, &rt, op));
    }
    attach_submit_key(node, runtime)
}

fn attach_submit_key(node: NodeBuilder, runtime: &TextFieldRuntime) -> NodeBuilder {
    let submit = runtime.clone();
    node.key(Shortcut::plain(NamedKey::Enter), move |event| {
        if event.phase != KeyPhase::Down {
            return;
        }
        event.stop_propagation();
        if !submit.disabled() {
            let value = submit.state.value().peek();
            (submit.submit)(value.as_str());
        }
    })
}

fn attach_clipboard_keys(node: NodeBuilder, runtime: &TextFieldRuntime) -> NodeBuilder {
    let pasted = runtime.clone();
    let node = node.paste(move |text| {
        if !pasted.disabled() {
            pasted.insert_pasted(text);
        }
        true
    });
    let commands: [KeyCommand; 4] = [
        ("c", Modifiers::empty(), clipboard_copy),
        ("x", Modifiers::empty(), clipboard_cut),
        ("v", Modifiers::empty(), clipboard_paste),
        ("a", Modifiers::empty(), |event, rt| {
            run_caret_key(event, rt, TextState::select_all);
        }),
    ];
    bind_command_keys(node, runtime, &commands)
}

fn attach_history_keys(node: NodeBuilder, runtime: &TextFieldRuntime) -> NodeBuilder {
    bind_history_keys(
        node,
        runtime.clone(),
        |event, rt| run_history(event, rt, false),
        |event, rt| run_history(event, rt, true),
    )
}

fn attach_text_input(node: NodeBuilder, runtime: &TextFieldRuntime) -> NodeBuilder {
    let rt = runtime.clone();
    node.key_any(move |event| {
        if event.phase != KeyPhase::Down {
            return;
        }
        if rt.disabled() {
            event.stop_propagation();
            return;
        }
        if let Some(text) = typed_event_text(event, false) {
            event.stop_propagation();
            rt.state.insert(&text);
            rt.sync_value();
        }
    })
}

fn attach_ime_and_a11y(node: NodeBuilder, runtime: &TextFieldRuntime) -> NodeBuilder {
    if let Some(disabled) = runtime.disabled {
        let rt = runtime.clone();
        effect(move || {
            if disabled.get() {
                rt.clear_preedit();
            }
        });
    }
    let anchor_rt = runtime.clone();
    let caret_anchor = memo(move || anchor_rt.caret_geometry());
    if let Some(target) = runtime.caret_rect {
        effect(move || {
            target.set(caret_anchor.get().map(|geom| {
                Rect::new(
                    crate::geometry::Point::new(geom.x(), geom.top()),
                    2.0,
                    geom.height(),
                )
            }));
        });
    }
    let disabled = runtime.disabled;
    let ime_rt = runtime.clone();
    let blur_rt = runtime.clone();
    let a11y_rt = runtime.clone();
    node.ime_eligible(move || !disabled.is_some_and(|flag| flag.peek()))
        .ime_caret(caret_anchor)
        .ime(move |event| ime_rt.handle_ime(event))
        .focus_out(move || blur_rt.clear_preedit())
        .a11y_edit(move |edit| a11y_rt.handle_a11y_edit(edit))
}

fn run_caret_key(event: &mut KeyEvent, runtime: &TextFieldRuntime, op: impl FnOnce(&TextState)) {
    run_caret_action(event, runtime, |rt| op(&rt.state));
}

fn run_caret_action(
    event: &mut KeyEvent,
    runtime: &TextFieldRuntime,
    op: impl FnOnce(&TextFieldRuntime),
) {
    if event.phase != KeyPhase::Down {
        return;
    }
    event.stop_propagation();
    if runtime.disabled() {
        return;
    }
    op(runtime);
    runtime.commit_caret();
}

fn run_edit_action(
    event: &mut KeyEvent,
    runtime: &TextFieldRuntime,
    op: impl FnOnce(&TextFieldRuntime),
) {
    if event.phase != KeyPhase::Down {
        return;
    }
    event.stop_propagation();
    if !runtime.disabled() {
        op(runtime);
    }
}

fn run_history(event: &mut KeyEvent, runtime: &TextFieldRuntime, redo: bool) {
    if event.phase != KeyPhase::Down {
        return;
    }
    event.stop_propagation();
    if runtime.disabled() {
        return;
    }
    let changed = if redo {
        runtime.state.redo()
    } else {
        runtime.state.undo()
    };
    if changed {
        runtime.sync_value();
    }
}

fn clipboard_copy(event: &mut KeyEvent, runtime: &TextFieldRuntime) {
    if event.phase != KeyPhase::Down {
        return;
    }
    if runtime.disabled() {
        event.stop_propagation();
        return;
    }
    if let Some(text) = runtime.state.selected_text() {
        event.prevent_default();
        event.stop_propagation();
        runtime.clip_fail.set(runtime.clipboard.set(&text).is_err());
    }
}

fn clipboard_cut(event: &mut KeyEvent, runtime: &TextFieldRuntime) {
    if event.phase != KeyPhase::Down {
        return;
    }
    event.stop_propagation();
    if runtime.disabled() {
        return;
    }
    if let Some(text) = runtime.state.selected_text() {
        let failed = runtime.clipboard.set(&text).is_err();
        runtime.clip_fail.set(failed);
        if failed {
            return;
        }
        runtime.state.backspace();
        runtime.sync_value();
    }
}

fn clipboard_paste(event: &mut KeyEvent, runtime: &TextFieldRuntime) {
    if event.phase != KeyPhase::Down {
        return;
    }
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

#[cfg(test)]
#[path = "handlers_tests.rs"]
mod tests;
