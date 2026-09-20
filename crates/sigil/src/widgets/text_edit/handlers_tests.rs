use std::cell::RefCell;
use std::iter::once;
use std::rc::Rc;

use super::{TextEditRuntime, TextEditState, noop_change};
use crate::events::EventRouter;
use crate::hal::{
    KeyEvent, KeyEventInit, KeyPhase, LogicalKey, LogicalSize, ModifiersState, PhysicalKey,
    ShellEvent, Size,
};
use crate::kernel::{Cx, TextContent};
use crate::layout::{LayoutEngine, TextMeasure};
use crate::reactive::{Runtime, signal};
use crate::widgets::clipboard::clipboard;
use crate::widgets::{ClipError, Clipboard, text_edit};

#[test]
fn external_caret_controls_the_next_edit() {
    Runtime::scope("external-caret", || {
        let value = signal(String::new());
        let caret = signal(0);
        let runtime = TextEditRuntime::new(
            TextEditState::new(value, caret),
            None,
            noop_change(),
            noop_change(),
            clipboard(),
            signal(true),
            crate::widgets::text_edit::render::content_padding(),
        );
        value.set("café".to_string());
        caret.set("café".len());
        let mut edit = runtime.edit_mut();
        edit.insert_str("\n");
        assert_eq!(edit.as_str(), "café\n");
        assert!(edit.undo());
        assert_eq!(edit.as_str(), "café");
    });
}

#[test]
fn inserted_text_undoes_and_redoes_after_rebuild() {
    for selected in [false, true] {
        Runtime::scope("insert-history", || {
            let original = "before 🦀 after ";
            let value = signal("stale".to_string());
            let caret = signal(5);
            let state = TextEditState::new(value, caret);
            value.set(original.to_string());
            caret.set(7);
            if selected {
                state.select(11, true);
            }
            state.insert("café 日本語 ");
            let inserted = if selected {
                "before café 日本語  after "
            } else {
                "before café 日本語 🦀 after "
            };
            assert_eq!(value.peek(), inserted);
            assert_eq!(caret.peek(), "before café 日本語 ".len());
            assert!(state.selection().is_none());
            for (key, modifiers, expected, position) in [
                (
                    "z",
                    ModifiersState::CONTROL,
                    original,
                    if selected { 11 } else { 7 },
                ),
                (
                    "Z",
                    ModifiersState::CONTROL | ModifiersState::SHIFT,
                    inserted,
                    "before café 日本語 ".len(),
                ),
            ] {
                let mut cx = Cx::new_root();
                let root = cx.add(text_edit(value).state(state.clone()));
                let mut layout = LayoutEngine::with_text_measure(Box::new(Measure));
                layout
                    .compute(&mut cx, root, Size::fixed(400.0, 200.0))
                    .expect("editor layout");
                let mut router = EventRouter::new();
                router.set_focus(&mut cx, Some(root));
                router.route(&mut cx, &layout, shortcut(key, modifiers));
                assert_eq!(value.peek(), expected);
                assert_eq!(caret.peek(), position);
            }
        });
    }
}

#[test]
fn replacement_undo_restores_the_original_caret_after_rebuild() {
    for replacement in ["edited café\n日本語 ", ""] {
        Runtime::scope("replacement-history", || {
            let original = "before 🦀 after ";
            let value = signal("stale".to_string());
            let caret = signal(5);
            let state = TextEditState::new(value, caret);
            value.set(original.to_string());
            caret.set(7);
            state.replace(replacement);
            state.replace(replacement);
            assert_eq!(value.peek(), replacement);
            assert_eq!(caret.peek(), replacement.len());
            for (key, modifiers, expected, position) in [
                ("z", ModifiersState::CONTROL, original, 7),
                (
                    "Z",
                    ModifiersState::CONTROL | ModifiersState::SHIFT,
                    replacement,
                    replacement.len(),
                ),
            ] {
                let mut cx = Cx::new_root();
                let root = cx.add(text_edit(value).state(state.clone()));
                let mut layout = LayoutEngine::with_text_measure(Box::new(Measure));
                layout
                    .compute(&mut cx, root, Size::fixed(400.0, 200.0))
                    .expect("editor layout");
                let mut router = EventRouter::new();
                router.set_focus(&mut cx, Some(root));
                router.route(&mut cx, &layout, shortcut(key, modifiers));
                assert_eq!(value.peek(), expected);
                assert_eq!(caret.peek(), position);
                assert!(state.selection().is_none());
            }
        });
    }
}

struct Memory(Rc<RefCell<String>>);

struct Measure;

impl TextMeasure for Measure {
    fn measure(&self, _content: &TextContent, _space: Size) -> LogicalSize {
        LogicalSize::new(200.0, 40.0)
    }
}

impl Clipboard for Memory {
    fn set(&self, text: &str) -> Result<(), ClipError> {
        *self.0.borrow_mut() = text.to_string();
        Ok(())
    }

    fn get(&self) -> Option<String> {
        Some(self.0.borrow().clone())
    }
}

#[test]
fn editing_shortcuts_copy_cut_paste_and_restore_unicode() {
    for modifiers in once(ModifiersState::CONTROL)
        .chain(cfg!(target_os = "macos").then_some(ModifiersState::META))
    {
        Runtime::scope("clipboard-shortcuts", || {
            let original = "café\n日本語";
            let value = signal(original.to_string());
            let state = TextEditState::new(value, signal(original.len()));
            let copied = Rc::new(RefCell::new(String::new()));
            for (key, shift, expected, clipboard) in [
                ("a", false, original, ""),
                ("c", false, original, original),
                ("x", false, "", original),
                ("v", false, original, original),
                ("z", false, "", original),
                ("Z", true, original, original),
            ] {
                let mut cx = Cx::new_root();
                let root = cx.add(
                    text_edit(value)
                        .state(state.clone())
                        .clipboard(Memory(copied.clone())),
                );
                let mut layout = LayoutEngine::with_text_measure(Box::new(Measure));
                layout
                    .compute(&mut cx, root, Size::fixed(400.0, 200.0))
                    .expect("editor layout");
                let mut router = EventRouter::new();
                router.set_focus(&mut cx, Some(root));
                router.route(
                    &mut cx,
                    &layout,
                    shortcut(
                        key,
                        modifiers
                            | if shift {
                                ModifiersState::SHIFT
                            } else {
                                ModifiersState::empty()
                            },
                    ),
                );
                assert_eq!(value.peek(), expected, "{modifiers:?} {key}: editor text");
                assert_eq!(
                    *copied.borrow(),
                    clipboard,
                    "{modifiers:?} {key}: clipboard text"
                );
            }
        });
    }
}

fn shortcut(key: &str, modifiers: ModifiersState) -> ShellEvent {
    ShellEvent::Key(KeyEvent::new(KeyEventInit {
        key: LogicalKey::character(key),
        physical: PhysicalKey::Unidentified,
        modifiers,
        repeat: false,
        phase: KeyPhase::Down,
        text: None,
    }))
}
