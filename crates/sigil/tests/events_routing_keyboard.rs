#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::indexing_slicing, clippy::panic, clippy::unwrap_used)]

#[path = "events_support/input.rs"]
mod input;
#[path = "events_support/mock_shell.rs"]
mod mock_shell;
#[path = "events_support/routing_helpers.rs"]
mod routing_helpers;

use std::cell::Cell;
use std::rc::Rc;

use sigil::events::prelude::*;
use sigil::hal::{Key, KeyPhase, LogicalKey, PhysicalKey, Shortcut};
use sigil::kernel::prelude::*;
use sigil::layout::prelude::*;

use input::{ctrl_s, key_event};
use routing_helpers::{compute_layout, route_event};

#[test]
fn shortcut_dispatch_invokes_action() {
    let saves = Rc::new(Cell::new(0_u32));
    let shortcut = Shortcut::ctrl(LogicalKey::character("s"));
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .layout(LayoutStyle::block().fixed_size(100.0, 100.0))
            .keyboard_shortcut(shortcut.clone())
            .key(shortcut, {
                let saves = Rc::clone(&saves);
                move |_| saves.set(saves.get() + 1)
            }),
    );
    let layout = compute_layout(&mut cx, root, 100.0, 100.0);
    let mut router = EventRouter::new();

    route_event(&mut router, &mut cx, &layout, ctrl_s());

    assert_eq!(saves.get(), 1);
}

#[test]
fn focused_key_up_does_not_invoke_action() {
    let hits = Rc::new(Cell::new(0_u32));
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .focusable(true)
            .layout(LayoutStyle::block().fixed_size(100.0, 100.0))
            .key(Key::Enter, {
                let hits = Rc::clone(&hits);
                move |_| hits.set(hits.get() + 1)
            }),
    );
    let layout = compute_layout(&mut cx, root, 100.0, 100.0);
    let mut router = EventRouter::new();
    router.set_focus(&mut cx, Some(root));

    route_event(
        &mut router,
        &mut cx,
        &layout,
        key_event(Key::Enter, PhysicalKey::Enter, KeyPhase::Up),
    );
    route_event(
        &mut router,
        &mut cx,
        &layout,
        key_event(Key::Enter, PhysicalKey::Enter, KeyPhase::Down),
    );

    assert_eq!(hits.get(), 1);
}

#[test]
fn focused_key_any_receives_unregistered_text_input() {
    let hits = Rc::new(Cell::new(0_u32));
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .focusable(true)
            .layout(LayoutStyle::block().fixed_size(100.0, 100.0))
            .key_any({
                let hits = Rc::clone(&hits);
                move |event| {
                    if event.text.as_deref() == Some("é") {
                        hits.set(hits.get() + 1);
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
        text_key_event("é", PhysicalKey::KeyE),
    );

    assert_eq!(hits.get(), 1);
}

fn text_key_event(text: &str, physical: PhysicalKey) -> sigil::hal::ShellEvent {
    sigil::hal::ShellEvent::Key(sigil::hal::KeyEvent::new(sigil::hal::KeyEventInit {
        key: LogicalKey::character(text),
        physical,
        modifiers: ModifiersState::empty(),
        repeat: false,
        phase: KeyPhase::Down,
        text: Some(text.into()),
    }))
}

struct SlashFixture {
    cx: Cx,
    layout: LayoutEngine,
    page: NodeId,
    text: NodeId,
    fires: Rc<Cell<u32>>,
    typed: Rc<Cell<u32>>,
}

fn slash_fixture() -> SlashFixture {
    let fires = Rc::new(Cell::new(0_u32));
    let typed = Rc::new(Cell::new(0_u32));
    let slash = Shortcut::plain(LogicalKey::character("/"));
    let mut cx = Cx::new_root();
    let page = cx.add(
        container()
            .focusable(true)
            .layout(LayoutStyle::block().fixed_size(100.0, 100.0))
            .keyboard_shortcut(slash.clone())
            .key(slash, {
                let fires = Rc::clone(&fires);
                move |_| fires.set(fires.get() + 1)
            }),
    );
    let text = cx.add(
        container()
            .role(sigil::kernel::Role::TextInput)
            .focusable(true)
            .layout(LayoutStyle::block().fixed_size(100.0, 20.0))
            .key_any({
                let typed = Rc::clone(&typed);
                move |event| {
                    if event.text.is_some() {
                        typed.set(typed.get() + 1);
                    }
                }
            }),
    );
    let layout = compute_layout(&mut cx, page, 100.0, 100.0);
    SlashFixture {
        cx,
        layout,
        page,
        text,
        fires,
        typed,
    }
}

#[test]
fn plain_char_shortcut_yields_to_focused_text_input() {
    let mut f = slash_fixture();
    let mut router = EventRouter::new();
    router.set_focus(&mut f.cx, Some(f.text));

    route_event(
        &mut router,
        &mut f.cx,
        &f.layout,
        text_key_event("/", PhysicalKey::Slash),
    );

    assert_eq!(
        f.fires.get(),
        0,
        "global '/' must not fire while a text input is focused"
    );
    assert_eq!(
        f.typed.get(),
        1,
        "the character falls through to the focused text input"
    );
}

#[test]
fn plain_char_shortcut_fires_when_non_text_node_focused() {
    let mut f = slash_fixture();
    let mut router = EventRouter::new();
    router.set_focus(&mut f.cx, Some(f.page));

    route_event(
        &mut router,
        &mut f.cx,
        &f.layout,
        text_key_event("/", PhysicalKey::Slash),
    );

    assert_eq!(f.fires.get(), 1, "'/' fires when no text input is focused");
    assert_eq!(f.typed.get(), 0);
}

#[test]
fn modifier_chord_still_fires_while_text_input_focused() {
    let fires = Rc::new(Cell::new(0_u32));
    let typed = Rc::new(Cell::new(0_u32));
    let chord = Shortcut::ctrl(LogicalKey::character("k"));
    let mut cx = Cx::new_root();
    let page = cx.add(
        container()
            .layout(LayoutStyle::block().fixed_size(100.0, 100.0))
            .keyboard_shortcut(chord.clone())
            .key(chord, {
                let fires = Rc::clone(&fires);
                move |_| fires.set(fires.get() + 1)
            }),
    );
    let text = cx.add(
        container()
            .role(sigil::kernel::Role::TextInput)
            .focusable(true)
            .layout(LayoutStyle::block().fixed_size(100.0, 20.0))
            .key_any({
                let typed = Rc::clone(&typed);
                move |event| {
                    if event.text.is_some() {
                        typed.set(typed.get() + 1);
                    }
                }
            }),
    );
    let layout = compute_layout(&mut cx, page, 100.0, 100.0);
    let mut router = EventRouter::new();
    router.set_focus(&mut cx, Some(text));

    route_event(&mut router, &mut cx, &layout, ctrl_k());

    assert_eq!(
        fires.get(),
        1,
        "Ctrl+K still fires globally while a text input is focused"
    );
    assert_eq!(typed.get(), 0);
}

fn ctrl_k() -> sigil::hal::ShellEvent {
    sigil::hal::ShellEvent::Key(sigil::hal::KeyEvent::new(sigil::hal::KeyEventInit {
        key: LogicalKey::character("k"),
        physical: PhysicalKey::KeyK,
        modifiers: ModifiersState::CONTROL,
        repeat: false,
        phase: KeyPhase::Down,
        text: None,
    }))
}
