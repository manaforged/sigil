#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(missing_docs)]
// Integration tests intentionally keep assertions direct and use
// indexing for fixed test topology. See the hygiene framework.
#![allow(clippy::indexing_slicing, clippy::panic, clippy::unwrap_used)]

#[path = "support/mock_shell.rs"]
mod mock_shell;
#[path = "support/routing_helpers.rs"]
mod routing_helpers;

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use sigil_events::prelude::*;
use sigil_kernel::FocusScopeKind;
use sigil_kernel::prelude::*;
use sigil_layout::prelude::*;

use routing_helpers::{compute_layout, route_event};

const fn ime_event(event: ImeEvent) -> sigil_hal::ShellEvent {
    sigil_hal::ShellEvent::Ime(event)
}

/// Discriminant-only label so assertions don't depend on `ImeEvent`'s
/// `Debug` formatting of its `SmolStr`/tuple payloads.
const fn ime_label(event: &ImeEvent) -> &'static str {
    match event {
        ImeEvent::Enabled => "Enabled",
        ImeEvent::Preedit(..) => "Preedit",
        ImeEvent::Commit(_) => "Commit",
        ImeEvent::Disabled => "Disabled",
    }
}

#[test]
fn route_ime_delivers_preedit_to_focused_target() {
    let seen = Rc::new(RefCell::new(String::new()));
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .focusable(true)
            .layout(LayoutStyle::block().fixed_size(100.0, 100.0))
            .on_ime({
                let seen = Rc::clone(&seen);
                move |event| {
                    if let ImeEvent::Preedit(text, _) = event {
                        *seen.borrow_mut() = text.to_string();
                    }
                }
            }),
    );
    let layout = compute_layout(&mut cx, root, 100.0, 100.0);
    let mut router = EventRouter::new();
    router.set_focus(&mut cx, Some(root));

    route_event(
        &mut router,
        &mut cx,
        &layout,
        ime_event(ImeEvent::Preedit("か".into(), Some((0, 3)))),
    );

    assert_eq!(*seen.borrow(), "か");
}

#[test]
fn route_ime_delivers_commit_to_focused_target() {
    let committed = Rc::new(RefCell::new(String::new()));
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .focusable(true)
            .layout(LayoutStyle::block().fixed_size(100.0, 100.0))
            .on_ime({
                let committed = Rc::clone(&committed);
                move |event| {
                    if let ImeEvent::Commit(text) = event {
                        committed.borrow_mut().push_str(text);
                    }
                }
            }),
    );
    let layout = compute_layout(&mut cx, root, 100.0, 100.0);
    let mut router = EventRouter::new();
    router.set_focus(&mut cx, Some(root));

    // A session must open (Enabled/Preedit) before Commit is a valid
    // continuation — a bare Commit with no owner is dropped by design
    // (W5-IME HIGH-1: never adopt a fresh owner from Commit/Disabled).
    route_event(&mut router, &mut cx, &layout, ime_event(ImeEvent::Enabled));
    route_event(
        &mut router,
        &mut cx,
        &layout,
        ime_event(ImeEvent::Commit("か".into())),
    );

    assert_eq!(*committed.borrow(), "か");
}

#[test]
fn route_ime_dropped_when_no_focus() {
    let hits = Rc::new(Cell::new(0_u32));
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .focusable(true)
            .layout(LayoutStyle::block().fixed_size(100.0, 100.0))
            .on_ime({
                let hits = Rc::clone(&hits);
                move |_| hits.set(hits.get() + 1)
            }),
    );
    let layout = compute_layout(&mut cx, root, 100.0, 100.0);
    let mut router = EventRouter::new();
    // No `set_focus` call: nothing is focused.

    route_event(
        &mut router,
        &mut cx,
        &layout,
        ime_event(ImeEvent::Preedit("x".into(), None)),
    );

    assert_eq!(hits.get(), 0, "no panic and no dispatch without focus");
}

#[test]
fn route_ime_dropped_when_focused_node_has_no_ime_target() {
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .focusable(true)
            .layout(LayoutStyle::block().fixed_size(100.0, 100.0)),
    );
    let layout = compute_layout(&mut cx, root, 100.0, 100.0);
    let mut router = EventRouter::new();
    router.set_focus(&mut cx, Some(root));

    // Focused, but never called `.on_ime(...)` — must not panic.
    route_event(
        &mut router,
        &mut cx,
        &layout,
        ime_event(ImeEvent::Commit("x".into())),
    );

    assert!(!router.focus_wants_ime(&cx));
}

#[test]
fn focus_wants_ime_reflects_on_ime_registration() {
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .focusable(true)
            .layout(LayoutStyle::block().fixed_size(100.0, 100.0))
            .on_ime(|_| {}),
    );
    let _layout = compute_layout(&mut cx, root, 100.0, 100.0);
    let mut router = EventRouter::new();

    assert!(!router.focus_wants_ime(&cx), "unfocused: no IME target yet");

    router.set_focus(&mut cx, Some(root));

    assert!(router.focus_wants_ime(&cx));
}

type ImeLog = Rc<RefCell<Vec<&'static str>>>;

/// Two side-by-side focusable IME targets, each logging the
/// discriminant of every `ImeEvent` it receives.
fn two_logging_ime_fields(cx: &mut Cx) -> (NodeId, NodeId, ImeLog, ImeLog) {
    let a_log = ImeLog::default();
    let b_log = ImeLog::default();
    let root = cx.add(
        container()
            .layout(LayoutStyle::flex_row().fixed_size(200.0, 100.0))
            .child(
                container()
                    .focusable(true)
                    .layout(LayoutStyle::block().fixed_size(100.0, 100.0))
                    .on_ime({
                        let log = Rc::clone(&a_log);
                        move |event| log.borrow_mut().push(ime_label(event))
                    }),
            )
            .child(
                container()
                    .focusable(true)
                    .layout(LayoutStyle::block().fixed_size(100.0, 100.0))
                    .on_ime({
                        let log = Rc::clone(&b_log);
                        move |event| log.borrow_mut().push(ime_label(event))
                    }),
            ),
    );
    let a = cx.nth_child(root, 0).expect("field A");
    let b = cx.nth_child(root, 1).expect("field B");
    (a, b, a_log, b_log)
}

#[test]
fn route_ime_focus_change_cancels_owner_and_drops_commit() {
    // W5-IME HIGH-1 repro: Preedit(A) -> focus moves to B -> Commit must
    // be a no-op on B, and A must see its session cancelled instead of
    // being left with a stale preedit.
    let mut cx = Cx::new_root();
    let (a, b, a_log, b_log) = two_logging_ime_fields(&mut cx);
    let root = cx.parent(a).expect("field A has a parent");
    let layout = compute_layout(&mut cx, root, 200.0, 100.0);
    let mut router = EventRouter::new();
    router.set_focus(&mut cx, Some(a));

    route_event(
        &mut router,
        &mut cx,
        &layout,
        ime_event(ImeEvent::Preedit("か".into(), Some((0, 3)))),
    );
    router.set_focus(&mut cx, Some(b));
    route_event(
        &mut router,
        &mut cx,
        &layout,
        ime_event(ImeEvent::Commit("か".into())),
    );

    assert!(
        b_log.borrow().is_empty(),
        "Commit must never land on the newly-focused node: {:?}",
        b_log.borrow()
    );
    assert_eq!(
        *a_log.borrow(),
        vec!["Preedit", "Disabled"],
        "the old owner must see its session cancelled (synthetic Disabled), not silently dropped"
    );
}

#[test]
fn route_ime_preedit_after_focus_change_stays_quarantined_until_enabled() {
    // final review HIGH: after a focus change cancels the session,
    // the SAME composition's follow-up Preedit/Commit must NOT adopt the
    // newly focused node as a fresh owner — only an OS lifecycle
    // boundary (Disabled then Enabled) may open a new session.
    let mut cx = Cx::new_root();
    let (a, b, a_log, b_log) = two_logging_ime_fields(&mut cx);
    let root = cx.parent(a).expect("field A has a parent");
    let layout = compute_layout(&mut cx, root, 200.0, 100.0);
    let mut router = EventRouter::new();
    router.set_focus(&mut cx, Some(a));

    route_event(
        &mut router,
        &mut cx,
        &layout,
        ime_event(ImeEvent::Preedit("か".into(), Some((0, 3)))),
    );
    router.set_focus(&mut cx, Some(b));
    // The in-flight composition keeps streaming: neither event may reach B.
    route_event(
        &mut router,
        &mut cx,
        &layout,
        ime_event(ImeEvent::Preedit("かん".into(), Some((0, 6)))),
    );
    route_event(
        &mut router,
        &mut cx,
        &layout,
        ime_event(ImeEvent::Commit("感".into())),
    );
    assert!(
        b_log.borrow().is_empty(),
        "quarantined composition leaked to the new focus: {:?}",
        b_log.borrow()
    );

    // OS lifecycle boundary: Disabled closes the old session, Enabled +
    // Preedit open a legitimate new one on B.
    route_event(&mut router, &mut cx, &layout, ime_event(ImeEvent::Disabled));
    route_event(&mut router, &mut cx, &layout, ime_event(ImeEvent::Enabled));
    route_event(
        &mut router,
        &mut cx,
        &layout,
        ime_event(ImeEvent::Preedit("に".into(), Some((0, 3)))),
    );
    assert_eq!(
        *b_log.borrow(),
        vec!["Enabled", "Preedit"],
        "a fresh session after the lifecycle boundary must reach B"
    );
    assert_eq!(
        *a_log.borrow(),
        vec!["Preedit", "Disabled"],
        "A sees exactly its own session: opening preedit + cancellation"
    );
}

#[test]
fn route_ime_scope_change_cancels_owner_without_focus_move() {
    // A composing field stays literally focused, but a modal scope opens
    // ABOVE it without moving focus into the modal — W5-IME HIGH-1's
    // "inert modal opening over a composing field" case. The owner falls
    // outside the new active scope, so the session must still cancel.
    let field_log = Rc::new(RefCell::new(Vec::new()));
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .layout(LayoutStyle::block().fixed_size(200.0, 200.0))
            .child(
                container()
                    .focusable(true)
                    .layout(LayoutStyle::block().fixed_size(100.0, 100.0))
                    .on_ime({
                        let log = Rc::clone(&field_log);
                        move |event| log.borrow_mut().push(ime_label(event))
                    }),
            ),
    );
    let field = cx.nth_child(root, 0).expect("composing field");
    let mut layout = compute_layout(&mut cx, root, 200.0, 200.0);
    let mut router = EventRouter::new();
    router.set_focus(&mut cx, Some(field));

    route_event(
        &mut router,
        &mut cx,
        &layout,
        ime_event(ImeEvent::Preedit("か".into(), Some((0, 3)))),
    );

    // Open a modal scope that does NOT contain `field` and does NOT
    // steal focus (the composing field is still `router.focus()`).
    let _modal = cx.add_child(
        root,
        container()
            .mutate(|node| node.a11y.focus_scope = Some(FocusScopeKind::Modal))
            .layout(LayoutStyle::block().fixed_size(50.0, 50.0)),
    );
    layout = compute_layout(&mut cx, root, 200.0, 200.0);

    route_event(
        &mut router,
        &mut cx,
        &layout,
        ime_event(ImeEvent::Commit("か".into())),
    );

    assert_eq!(
        router.focus(),
        Some(field),
        "opening the scope doesn't itself move focus"
    );
    assert_eq!(
        *field_log.borrow(),
        vec!["Preedit", "Disabled"],
        "field fell outside the new active scope — its session must cancel and Commit must not land"
    );
}
