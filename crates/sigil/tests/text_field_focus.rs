#![allow(clippy::expect_used, clippy::panic)]

use crate::widget_support;

use sigil::events::EventRouter;
use sigil::events::PhysicalKey;
use sigil::events::{Event, KeyPhase};
use sigil::geometry::Constraint;
use sigil::kernel::{Cx, NodeId};
use sigil::layout::LayoutStyle;
use sigil::prelude::*;
use widget_support::compute;

const VIEWPORT: Constraint = Constraint::fixed(320.0, 120.0);

fn key(ch: &str, physical: PhysicalKey, phase: KeyPhase) -> Event {
    Event::key(Key::character(ch), phase)
        .physical(physical)
        .text((phase == KeyPhase::Down).then(|| ch.into()))
}

fn field_node(cx: &Cx, root: NodeId) -> NodeId {
    let mut stack = vec![root];
    while let Some(id) = stack.pop() {
        let node = cx.get(id).expect("node alive");
        if node.a11y.role == Role::TextInput {
            return id;
        }
        stack.extend(node.children().iter().copied());
    }
    panic!("no text field in the tree");
}

#[test]
fn focus_when_bump_focuses_the_field() {
    sigil::reactive::Owner::scope("text-field-focus-when", || {
        let value = signal(String::new());
        let trigger = signal(0_u64);
        let mut cx = Cx::new_root();
        let root = cx.add(text_field(value).focus_when(trigger));
        let layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
        let mut router = EventRouter::new();

        router.flush_focus_requests(&mut cx, &layout);
        assert_eq!(router.focus(), None);

        trigger.set(1);
        router.flush_focus_requests(&mut cx, &layout);
        assert_eq!(router.focus(), Some(root));
    });
}

#[test]
fn autofocus_field_takes_focus_on_mount() {
    sigil::reactive::Owner::scope("text-field-focus-mount", || {
        let value = signal(String::new());
        let mut cx = Cx::new_root();
        let root = cx.add(text_field(value).autofocus());
        let layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
        let mut router = EventRouter::new();

        router.flush_focus_requests(&mut cx, &layout);
        assert_eq!(
            router.focus(),
            Some(root),
            "an autofocus field takes focus when it mounts"
        );
    });
}

#[test]
fn autofocus_inside_trapped_modal_takes_typing() {
    sigil::reactive::Owner::scope("text-field-focus-modal", || {
        let value = signal(String::new());
        let open = signal(true);
        let mut cx = Cx::new_root();
        let root = cx.add(
            container().child(
                modal(open)
                    .title("authentication")
                    .trap(true)
                    .child(text_field(value).autofocus()),
            ),
        );
        let layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
        let field = field_node(&cx, root);
        let mut router = EventRouter::new();

        router.flush_focus_requests(&mut cx, &layout);
        assert_eq!(
            router.focus(),
            Some(field),
            "the mounted field holds focus inside the trapped modal"
        );

        router.route(
            &mut cx,
            &layout,
            key("a", PhysicalKey::KeyA, KeyPhase::Down),
        );
        router.route(&mut cx, &layout, key("a", PhysicalKey::KeyA, KeyPhase::Up));
        assert_eq!(value.get(), "a", "typing reaches the mounted field");
    });
}

#[test]
fn slash_shortcut_focuses_field_then_typing_reaches_it() {
    sigil::reactive::Owner::scope("text-field-slash-e2e", || {
        let value = signal(String::new());
        let trigger = signal(0_u64);
        let mut cx = Cx::new_root();
        let root = cx.add(
            container()
                .layout(LayoutStyle::block().fixed_size(320.0, 120.0))
                .keyboard_shortcut(Shortcut::plain(Key::character("/")))
                .key(Shortcut::plain(Key::character("/")), move |_| {
                    trigger.set(trigger.peek() + 1);
                })
                .child(text_field(value).focus_when(trigger)),
        );
        let layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
        let field = field_node(&cx, root);
        let mut router = EventRouter::new();

        router.route(
            &mut cx,
            &layout,
            key("/", PhysicalKey::Slash, KeyPhase::Down),
        );
        assert_eq!(trigger.get(), 1, "page '/' shortcut fires");
        router.route(&mut cx, &layout, key("/", PhysicalKey::Slash, KeyPhase::Up));
        router.flush_focus_requests(&mut cx, &layout);
        assert_eq!(
            router.focus(),
            Some(field),
            "focus request applies at end of frame"
        );

        router.route(
            &mut cx,
            &layout,
            key("/", PhysicalKey::Slash, KeyPhase::Down),
        );
        assert_eq!(trigger.get(), 1, "shortcut must not re-fire while typing");
        assert_eq!(value.get(), "/");
    });
}
