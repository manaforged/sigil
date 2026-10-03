#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use sigil::Host;
use sigil::app::FrameHooks;
use sigil::events::{EventRouter, KeyEventInit, PhysicalKey, ShellEvent};
use sigil::prelude::*;
use sigil::reactive::Runtime;
use sigil::widgets::DiffBuilder;

const VIEW: (f32, f32) = (480.0, 320.0);

fn settle(host: &mut Host, router: &mut EventRouter) {
    for _ in 0..3 {
        let _ = host.frame(router, VIEW, 1.0 / 60.0, FrameHooks::default());
    }
}

fn press(key: Key, modifiers: ModifiersState) -> ShellEvent {
    ShellEvent::Key(KeyEvent::new(KeyEventInit {
        key: LogicalKey::Named(key),
        physical: PhysicalKey::Enter,
        modifiers,
        repeat: false,
        phase: KeyPhase::Down,
        text: None,
    }))
}

fn send(host: &mut Host, router: &mut EventRouter, keys: Vec<ShellEvent>) {
    let queue = RefCell::new(Some(keys));
    let _ = host.frame(
        router,
        VIEW,
        1.0 / 60.0,
        FrameHooks {
            input: Some(Box::new(move |target, events| {
                for event in queue.borrow_mut().take().unwrap_or_default() {
                    target.route(events, event);
                }
                None
            })),
            ..FrameHooks::default()
        },
    );
    settle(host, router);
}

#[test]
fn focus_when_focuses_any_widget_each_time_its_signal_changes() {
    Runtime::scope("api_fixes::focus_when", || {
        let trigger = signal(0_u64);
        let focused = Rc::new(Cell::new(0_u32));
        let count = Rc::clone(&focused);
        let mut host = Host::new();
        let mut router = EventRouter::new();
        host.mount(
            column().child(button("First").autofocus()).child(
                button("Target")
                    .focus_when(trigger)
                    .focus_in(move || count.set(count.get() + 1)),
            ),
        );
        settle(&mut host, &mut router);
        assert_eq!(focused.get(), 0);

        trigger.set(1);
        settle(&mut host, &mut router);

        assert_eq!(focused.get(), 1);
    });
}

#[test]
fn submit_on_enter_sends_on_enter_and_breaks_lines_on_shift_enter() {
    Runtime::scope("api_fixes::submit_on", || {
        let draft = signal(String::from("Hi"));
        let sent = Rc::new(RefCell::new(Vec::<String>::new()));
        let log = Rc::clone(&sent);
        let mut host = Host::new();
        let mut router = EventRouter::new();
        host.mount(
            column().child(
                text_edit(draft)
                    .submit_on(SubmitKey::Enter)
                    .submit(move |value| log.borrow_mut().push(value.to_owned()))
                    .autofocus(),
            ),
        );
        settle(&mut host, &mut router);

        send(
            &mut host,
            &mut router,
            vec![press(Key::Enter, ModifiersState::SHIFT)],
        );
        assert!(sent.borrow().is_empty());
        assert!(draft.get().contains('\n'), "{:?}", draft.get());

        send(
            &mut host,
            &mut router,
            vec![press(Key::Enter, ModifiersState::empty())],
        );
        assert_eq!(sent.borrow().len(), 1);
    });
}

#[test]
fn a_cursor_row_moved_off_screen_scrolls_into_view() {
    Runtime::scope("api_fixes::cursor", || {
        let cursor = signal(0_usize);
        let scroll = signal(0.0_f32);
        let rows: Vec<String> = (0..50).map(|n| format!("Row {n}")).collect();
        let mut host = Host::new();
        let mut router = EventRouter::new();
        host.mount(
            column().child(
                virtual_list(rows, |_, row: &String| text(row.clone()))
                    .row_height(20.0)
                    .viewport_height(100.0)
                    .scroll_bind(scroll)
                    .cursor(cursor),
            ),
        );
        settle(&mut host, &mut router);
        assert_eq!(scroll.get(), 0.0);

        cursor.set(30);
        settle(&mut host, &mut router);

        assert!(scroll.get() > 0.0);
        assert!(host.emit(VIEW).find_text("Row 30").is_some());
    });
}

#[test]
fn a_chart_redraws_when_its_values_change() {
    Runtime::scope("api_fixes::chart", || {
        let values = signal(vec![1.0_f32, 1.0, 1.0]);
        let mut host = Host::new();
        let mut router = EventRouter::new();
        host.mount(column().child(chart(values)));
        settle(&mut host, &mut router);
        let before = host.emit(VIEW);

        values.set(vec![1.0, 9.0, 1.0]);
        settle(&mut host, &mut router);

        assert_ne!(host.emit(VIEW), before);
    });
}

fn checkboxes(host: &mut Host) -> Vec<bool> {
    let root = host.root().expect("mounted");
    let cx = host.cx();
    let mut found = Vec::new();
    cx.visit_preorder(root, |_, node| {
        if node.a11y.role == Role::CheckBox {
            found.push(node.a11y.checked.is_some_and(|checked| checked.get()));
        }
    });
    found
}

#[test]
fn markdown_task_items_render_as_check_boxes() {
    Runtime::scope("api_fixes::tasklist", || {
        let mut host = Host::new();
        let mut router = EventRouter::new();
        host.mount(column().child(markdown(signal(String::from(
            "- [ ] todo\n- [x] done\n- plain\n",
        )))));
        settle(&mut host, &mut router);

        assert_eq!(checkboxes(&mut host), vec![false, true]);
        assert!(host.emit(VIEW).find_text("todo").is_some());
        assert!(host.emit(VIEW).find_text("[x] done").is_none());
    });
}

#[test]
fn a_focused_empty_editor_keeps_its_placeholder() {
    Runtime::scope("api_fixes::placeholder", || {
        let mut host = Host::new();
        let mut router = EventRouter::new();
        host.mount(
            column().child(
                text_edit(signal(String::new()))
                    .placeholder("Message")
                    .autofocus(),
            ),
        );
        settle(&mut host, &mut router);

        assert!(host.emit(VIEW).find_text("Message").is_some());
    });
}

#[test]
fn the_prelude_names_what_its_functions_return() {
    Runtime::scope("api_fixes::prelude", || {
        let open = signal(false);
        let _: ModalBuilder = modal(open);
        let _: DiffBuilder = diff("--- a\n+++ b\n").view(DiffView::Unified);
        let _: Option<SortKey> = Some(SortKey {
            column: "name",
            dir: SortDir::Ascending,
        });
        let status = signal(Status::<u32, String>::Idle);
        let _ = match_status(status).pending(|| text("Loading"));
    });
}
