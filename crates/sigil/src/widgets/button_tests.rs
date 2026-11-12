#![allow(clippy::unwrap_used, clippy::panic)]

use super::*;
use crate::events::NamedKey;
use crate::events::{Event, EventRouter, Key, KeyPhase, MouseButton, MouseButtons, PointerPhase};
use crate::geometry::{Point, Size};
use crate::kernel::{AccessibleName, Cx, IntoNode, Role};
use crate::layout::LayoutEngine;
use crate::reactive::{Owner, signal};

#[test]
fn disabled_button_recovers_and_blocks_repeat_activation() {
    Owner::scope("button-disabled-transition", || {
        let disabled = signal(true);
        let pending = signal(false);
        let clicks = signal(0);
        let mut cx = Cx::new_root();
        let id = cx.add(
            button("Save")
                .disabled(disabled)
                .pending(pending)
                .activate(move || clicks.set(clicks.peek() + 1)),
        );
        let mut layout = LayoutEngine::with_text_measure(Box::new(
            crate::layout::FakeMeasure::fixed(80.0, 20.0),
        ));
        layout
            .compute(&mut cx, id, Size::fixed(160.0, 40.0))
            .expect("layout");
        let mut router = EventRouter::new();
        for (invalid, busy, enabled, expected) in [
            (true, false, false, 0),
            (false, false, true, 2),
            (false, true, false, 2),
            (false, false, true, 4),
            (true, false, false, 4),
        ] {
            disabled.set(invalid);
            pending.set(busy);
            router.set_focus(&mut cx, None);
            router.focus_next(&mut cx, &layout);
            assert_eq!(router.focus(), enabled.then_some(id));
            assert_eq!(router.is_focus_target(&cx, &layout, id), enabled);
            assert_eq!(router.is_click_target(&cx, &layout, id), enabled);
            click(&mut router, &mut cx, &layout);
            router.set_focus(&mut cx, Some(id));
            press(&mut router, &mut cx, &layout, NamedKey::Enter);
            assert_eq!(clicks.peek(), expected);
            router.set_focus(&mut cx, None);
            router.focus_prev(&mut cx, &layout);
            assert_eq!(router.focus(), enabled.then_some(id));
        }
    });
}

fn click(router: &mut EventRouter, cx: &mut Cx, layout: &LayoutEngine) {
    for phase in [PointerPhase::Move, PointerPhase::Down, PointerPhase::Up] {
        router.route(
            cx,
            layout,
            Event::pointer(phase, Point::new(10.0, 10.0))
                .button(Some(MouseButton::Left))
                .buttons(MouseButtons::LEFT),
        );
    }
}

fn press(router: &mut EventRouter, cx: &mut Cx, layout: &LayoutEngine, key: NamedKey) {
    router.route(cx, layout, Event::key(Key::Named(key), KeyPhase::Down));
}

#[test]
fn icon_button_names_itself_after_the_glyph_unless_told() {
    Owner::scope("icon-button-role", || {
        let mut cx = Cx::new_root();
        let named = button("\u{25B6}")
            .ghost()
            .square()
            .activate(|| {})
            .accessible("Play");
        for (button, expected) in [
            (named, "Play"),
            (
                button("\u{25B6}").ghost().square().activate(|| {}),
                "\u{25B6}",
            ),
        ] {
            let id = button.build(&mut cx);
            let node = cx.get(id).expect("button");
            assert_eq!(node.a11y.role, Role::Button);
            match &node.a11y.name {
                AccessibleName::Static(name) => assert_eq!(name, expected),
                other => panic!("expected static accessible name, got {other:?}"),
            }
        }
    });
}

#[test]
fn pending_button_starts_disabled() {
    Owner::scope("button-pending", || {
        let busy = signal(true);
        let mut cx = Cx::new_root();
        let id = button("Save").pending(busy).build(&mut cx);
        let node = cx.get(id).expect("button");
        assert!(!node.a11y.is_focusable(), "pending button is not focusable");
    });
}
