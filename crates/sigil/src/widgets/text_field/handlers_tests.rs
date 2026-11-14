use std::cell::RefCell;
use std::iter::once;
use std::rc::Rc;

use crate::events::EventRouter;
use crate::events::{Event, ImeEvent, Key, KeyPhase, Modifiers, NamedKey};
use crate::geometry::Constraint;
use crate::kernel::{Cx, IntoNode, NodeId, container};
use crate::layout::LayoutEngine;
use crate::reactive::{Owner, signal};

use crate::Error;
use crate::clipboard::Clipboard;
use crate::widgets::{TextFieldBuilder, TextState, text_field};

struct Memory(Rc<RefCell<String>>);

impl Clipboard for Memory {
    fn get(&self) -> Option<String> {
        Some(self.0.borrow().clone())
    }

    fn set(&self, text: &str) -> Result<crate::clipboard::Delivery, Error> {
        self.0.replace(text.to_string());
        Ok(crate::clipboard::Delivery::Confirmed)
    }
}

struct Mounted {
    cx: Cx,
    layout: LayoutEngine,
    router: EventRouter,
    root: NodeId,
}

impl Mounted {
    fn new(node: impl IntoNode) -> Self {
        let mut cx = Cx::new_root();
        let root = cx.add(node);
        let mut layout = LayoutEngine::new(Box::new(crate::layout::FakeMeasure::fixed(80.0, 20.0)));
        layout
            .compute(&mut cx, root, Constraint::fixed(400.0, 200.0))
            .expect("field layout");
        Self {
            cx,
            layout,
            router: EventRouter::new(),
            root,
        }
    }

    fn focused(node: impl IntoNode) -> Self {
        let mut mounted = Self::new(node);
        mounted.focus(Some(mounted.root));
        mounted
    }

    fn focus(&mut self, target: Option<NodeId>) {
        self.router.set_focus(&mut self.cx, target);
    }

    fn send(&mut self, event: Event) {
        self.router.route(&mut self.cx, &self.layout, event);
    }
}

type Changes = Rc<RefCell<Vec<String>>>;

fn recorded(state: TextState, multiline: bool) -> (TextFieldBuilder, Changes) {
    let changes = Changes::default();
    let seen = Rc::clone(&changes);
    let field = text_field(state).change(move |text| seen.borrow_mut().push(text.to_string()));
    let field = if multiline { field.multiline() } else { field };
    (field, changes)
}

#[test]
fn shortcuts_copy_cut_paste_and_restore_unicode() {
    for modifiers in
        once(Modifiers::CTRL).chain(cfg!(target_os = "macos").then_some(Modifiers::META))
    {
        Owner::scope("field-shortcuts", || {
            let original = "café 日本語";
            let value = signal(original.to_string());
            let copied = Rc::new(RefCell::new(String::new()));
            let mut field = Mounted::focused(text_field(value).clipboard(Memory(copied.clone())));
            let plain = Modifiers::empty();
            for (key, shift, expected, clipboard) in [
                ("a", plain, original, ""),
                ("c", plain, original, original),
                ("x", plain, "", original),
                ("v", plain, original, original),
                ("z", plain, "", original),
                ("Z", Modifiers::SHIFT, original, original),
            ] {
                field.send(
                    Event::key(Key::character(key), KeyPhase::Down).modifiers(modifiers | shift),
                );
                assert_eq!(value.peek(), expected, "{modifiers:?} {key}: field text");
                assert_eq!(
                    *copied.borrow(),
                    clipboard,
                    "{modifiers:?} {key}: clipboard"
                );
            }
        });
    }
}

#[test]
fn tab_typed_in_a_multiline_field_inserts_and_keeps_focus() {
    Owner::scope("multiline-tab", || {
        let value = signal(String::new());
        let mut form = Mounted::new(
            container()
                .child(text_field(value).multiline())
                .child(text_field(signal(String::new()))),
        );
        let editor = form.cx.first_child(form.root);
        assert!(editor.is_some(), "the editor mounts under the container");
        form.focus(editor);
        form.send(Event::key(NamedKey::Tab, KeyPhase::Down).text(Some("\t".to_string())));
        assert_eq!(value.peek(), "\t");
        assert_eq!(
            form.router.focus(),
            editor,
            "a Tab the editor typed must not also move focus"
        );
    });
}

#[test]
fn edits_that_keep_the_text_do_not_call_change() {
    for multiline in [false, true] {
        Owner::scope("field-noop-change", || {
            let value = signal("abc".to_string());
            let (field, changes) = recorded(TextState::from(value), multiline);
            let mut field = Mounted::focused(field);
            field.send(Event::key(NamedKey::Delete, KeyPhase::Down));
            assert_eq!(value.peek(), "abc");
            assert!(
                changes.borrow().is_empty(),
                "multiline={multiline}: Delete at the end changed nothing"
            );
            field.send(Event::key(NamedKey::Backspace, KeyPhase::Down));
            assert_eq!(value.peek(), "ab");
            assert_eq!(*changes.borrow(), ["ab"], "multiline={multiline}");
        });
    }
}

#[test]
fn line_edge_deletes_remove_the_selection() {
    for multiline in [false, true] {
        for key in ["k", "u"] {
            Owner::scope("field-edge-delete", || {
                let value = signal("hello world".to_string());
                let state = TextState::from(value);
                state.select(2);
                state.extend(5);
                let (field, changes) = recorded(state.clone(), multiline);
                let mut field = Mounted::focused(field);
                field.send(
                    Event::key(Key::character(key), KeyPhase::Down).modifiers(Modifiers::CTRL),
                );
                assert_eq!(value.peek(), "he world", "multiline={multiline} Ctrl+{key}");
                assert!(state.selection().is_none());
                assert_eq!(*changes.borrow(), ["he world"]);
            });
        }
    }
}

#[test]
fn empty_ime_commit_leaves_the_field_alone() {
    for multiline in [false, true] {
        Owner::scope("field-empty-commit", || {
            let value = signal("abc".to_string());
            let state = TextState::from(value);
            state.select(1);
            state.extend(2);
            let (field, changes) = recorded(state.clone(), multiline);
            let mut field = Mounted::focused(field);
            field.send(Event::Ime(ImeEvent::Enabled));
            field.send(Event::Ime(ImeEvent::Commit(String::new())));
            assert_eq!(value.peek(), "abc");
            assert_eq!(state.selection(), Some(1..2), "multiline={multiline}");
            assert!(changes.borrow().is_empty(), "multiline={multiline}");
            field.send(Event::Ime(ImeEvent::Commit("é".to_string())));
            assert_eq!(value.peek(), "aéc");
            assert_eq!(*changes.borrow(), ["aéc"], "multiline={multiline}");
        });
    }
}

#[test]
fn hidden_password_is_not_copied() {
    for key in ["c", "x"] {
        Owner::scope("pw-copy", || {
            let value = signal("hunter2".to_string());
            let copied = Rc::new(RefCell::new(String::new()));
            let mut field = Mounted::focused(
                text_field(value)
                    .password()
                    .clipboard(Memory(copied.clone())),
            );
            for k in ["a", key] {
                field
                    .send(Event::key(Key::character(k), KeyPhase::Down).modifiers(Modifiers::CTRL));
            }
            assert_eq!(*copied.borrow(), "", "Ctrl+{key} wrote the clipboard");
            assert_eq!(value.peek(), "hunter2", "Ctrl+{key} changed the text");
        });
    }
}

#[test]
fn paste_strips_control_characters_from_a_single_line_field() {
    Owner::scope("field-paste-tab", || {
        let value = signal(String::new());
        let copied = Rc::new(RefCell::new("a\tb".to_string()));
        let mut field = Mounted::focused(text_field(value).clipboard(Memory(copied)));
        field.send(Event::key(Key::character("v"), KeyPhase::Down).modifiers(Modifiers::CTRL));
        assert_eq!(value.peek(), "ab");
    });
}
