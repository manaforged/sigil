use std::cell::RefCell;
use std::iter::once;
use std::rc::Rc;

use super::{TextEditRuntime, TextEditState, noop_change};
use crate::clipboard::clipboard;
use crate::clipboard::{ClipError, Clipboard};
use crate::events::EventRouter;
use crate::events::{
    KeyEvent, KeyEventInit, KeyPhase, LogicalKey, ModifiersState, PhysicalKey, ShellEvent,
};
use crate::events::{NodeEventsExt, Shortcut};
use crate::geometry::{LogicalSize, Size};
use crate::kernel::{Cx, TextContent};
use crate::layout::{LayoutEngine, TextMeasure};
use crate::reactive::{Runtime, signal};
use crate::widgets::text_edit::{TextEditBuilder, text_edit};

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
fn edits_undo_and_redo_after_rebuild() {
    for (edit, edited, undone_at, edited_at) in history_cases() {
        Runtime::scope("edit-history", || {
            let original = "before 🦀 after ";
            let value = signal("stale".to_string());
            let caret = signal(5);
            let state = TextEditState::new(value, caret);
            value.set(original.to_string());
            caret.set(7);
            edit(&state);
            assert_eq!(value.peek(), edited);
            assert_eq!(caret.peek(), edited_at);
            assert!(state.selection().is_none());
            let redo = ModifiersState::CONTROL | ModifiersState::SHIFT;
            let steps = [
                ("z", ModifiersState::CONTROL, original, undone_at),
                ("Z", redo, edited, edited_at),
            ];
            for (key, modifiers, expected, position) in steps {
                press(text_edit(value).state(state.clone()), key, modifiers);
                assert_eq!(value.peek(), expected);
                assert_eq!(caret.peek(), position);
                assert!(state.selection().is_none());
            }
        });
    }
}

type HistoryCase = (fn(&TextEditState), &'static str, usize, usize);

fn history_cases() -> [HistoryCase; 4] {
    let typed = "before café 日本語 ".len();
    [
        (
            |state| state.insert("café 日本語 "),
            "before café 日本語 🦀 after ",
            7,
            typed,
        ),
        (
            |state| {
                state.select(11, true);
                state.insert("café 日本語 ");
            },
            "before café 日本語  after ",
            11,
            typed,
        ),
        (
            |state| (0..2).for_each(|_| state.replace("edited café\n日本語 ")),
            "edited café\n日本語 ",
            7,
            "edited café\n日本語 ".len(),
        ),
        (|state| (0..2).for_each(|_| state.replace("")), "", 7, 0),
    ]
}

struct Memory(Rc<RefCell<String>>);

struct Measure;

impl TextMeasure for Measure {
    fn measure(&self, _content: &TextContent, _space: Size) -> LogicalSize {
        LogicalSize::new(200.0, 40.0)
    }
}

impl Clipboard for Memory {
    fn set(&self, text: &str) -> Result<crate::clipboard::Delivery, ClipError> {
        *self.0.borrow_mut() = text.to_string();
        Ok(crate::clipboard::Delivery::Confirmed)
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
            let plain = ModifiersState::empty();
            for (key, shift, expected, clipboard) in [
                ("a", plain, original, ""),
                ("c", plain, original, original),
                ("x", plain, "", original),
                ("v", plain, original, original),
                ("z", plain, "", original),
                ("Z", ModifiersState::SHIFT, original, original),
            ] {
                let editor = text_edit(value)
                    .state(state.clone())
                    .clipboard(Memory(copied.clone()));
                press(editor, key, modifiers | shift);
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

struct Unreadable;

impl Clipboard for Unreadable {
    fn set(&self, _text: &str) -> Result<crate::clipboard::Delivery, ClipError> {
        Ok(crate::clipboard::Delivery::Requested)
    }

    fn get(&self) -> Option<String> {
        None
    }
}

#[test]
fn paste_without_a_readable_clipboard_leaves_the_key_to_the_host() {
    Runtime::scope("paste-unreadable", || {
        let value = signal("kept".to_string());
        let reached = signal(false);
        let mut cx = Cx::new_root();
        let root = cx.add(
            crate::kernel::container()
                .key(
                    Shortcut::new(LogicalKey::character("v"), ModifiersState::CONTROL),
                    move |_| reached.set(true),
                )
                .child(text_edit(value).clipboard(Unreadable)),
        );
        let mut layout = LayoutEngine::with_text_measure(Box::new(Measure));
        let laid = layout.compute(&mut cx, root, Size::fixed(400.0, 200.0));
        assert!(laid.is_ok(), "editor layout");
        let editor = cx.first_child(root);
        assert!(editor.is_some(), "the editor mounts under the container");
        let mut router = EventRouter::new();
        router.set_focus(&mut cx, editor);
        router.route(&mut cx, &layout, shortcut("v", ModifiersState::CONTROL));
        assert_eq!(value.peek(), "kept");
        assert!(
            reached.peek(),
            "Ctrl+V must reach the host when the clipboard cannot be read"
        );
    });
}

fn press(editor: TextEditBuilder, key: &str, modifiers: ModifiersState) {
    let mut cx = Cx::new_root();
    let root = cx.add(editor);
    let mut layout = LayoutEngine::with_text_measure(Box::new(Measure));
    layout
        .compute(&mut cx, root, Size::fixed(400.0, 200.0))
        .expect("editor layout");
    let mut router = EventRouter::new();
    router.set_focus(&mut cx, Some(root));
    router.route(&mut cx, &layout, shortcut(key, modifiers));
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
