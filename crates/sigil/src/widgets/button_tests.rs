#![allow(clippy::unwrap_used, clippy::panic)]

use super::*;
use crate::events::EventRouter;
use crate::events::{
    KeyEvent, KeyEventInit, KeyPhase, LogicalKey, ModifiersState, MouseButton, MouseButtons,
    PhysicalKey, PointerEvent, PointerEventInit, PointerKind, PointerPhase, ShellEvent,
};
use crate::geometry::{LogicalSize, Point, Size};
use crate::kernel::IntoNode;
use crate::kernel::{AccessibleName, Cx, Role};
use crate::layout::{LayoutEngine, TextMeasure};
use crate::reactive::Runtime;
use crate::reactive::signal;

struct Measure;

impl TextMeasure for Measure {
    fn measure(&self, _source: &TextContent, _space: Size) -> LogicalSize {
        LogicalSize::new(80.0, 20.0)
    }
}

#[test]
fn disabled_button_recovers_and_blocks_repeat_activation() {
    Runtime::scope("button-disabled-transition", || {
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
        let mut layout = LayoutEngine::with_text_measure(Box::new(Measure));
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
            router.focus_next(&mut cx);
            assert_eq!(router.focus(), enabled.then_some(id));
            assert_eq!(router.is_focus_target(&cx, &layout, id), enabled);
            assert_eq!(router.is_click_target(&cx, &layout, id), enabled);
            click(&mut router, &mut cx, &layout);
            router.set_focus(&mut cx, Some(id));
            press(&mut router, &mut cx, &layout, Key::Enter);
            assert_eq!(clicks.peek(), expected);
            router.set_focus(&mut cx, None);
            router.focus_prev(&mut cx);
            assert_eq!(router.focus(), enabled.then_some(id));
        }
    });
}

fn click(router: &mut EventRouter, cx: &mut Cx, layout: &LayoutEngine) {
    for phase in [PointerPhase::Move, PointerPhase::Down, PointerPhase::Up] {
        router.route(
            cx,
            layout,
            ShellEvent::Pointer(PointerEvent::new(PointerEventInit {
                position: Point::new(10.0, 10.0),
                delta: Point::new(0.0, 0.0),
                kind: PointerKind::Mouse,
                button: Some(MouseButton::Left),
                buttons: MouseButtons::LEFT,
                modifiers: ModifiersState::empty(),
                phase,
            })),
        );
    }
}

fn press(router: &mut EventRouter, cx: &mut Cx, layout: &LayoutEngine, key: Key) {
    router.route(
        cx,
        layout,
        ShellEvent::Key(KeyEvent::new(KeyEventInit {
            key: LogicalKey::Named(key),
            physical: PhysicalKey::Unidentified,
            modifiers: ModifiersState::empty(),
            repeat: false,
            phase: KeyPhase::Down,
            text: None,
        })),
    );
}

#[test]
fn icon_button_is_a_ghost_button() {
    Runtime::scope("icon-button-role", || {
        let mut cx = Cx::new_root();
        let id = icon_button("\u{25B6}", || {})
            .accessible("Play")
            .build(&mut cx);
        let node = cx.get(id).expect("button");
        assert_eq!(node.a11y.role, Role::Button);
        match &node.a11y.name {
            AccessibleName::Static(name) => assert_eq!(name, "Play"),
            other => panic!("expected static accessible name, got {other:?}"),
        }
    });
}

#[test]
fn icon_button_defaults_accessible_name_to_glyph() {
    Runtime::scope("icon-button-glyph-name", || {
        let mut cx = Cx::new_root();
        let id = icon_button("\u{25B6}", || {}).build(&mut cx);
        let node = cx.get(id).expect("button");
        match &node.a11y.name {
            AccessibleName::Static(name) => assert_eq!(name, "\u{25B6}"),
            other => panic!("expected static name, got {other:?}"),
        }
    });
}

#[test]
fn pending_button_starts_disabled() {
    Runtime::scope("button-pending", || {
        let busy = crate::reactive::signal(true);
        let mut cx = Cx::new_root();
        let id = button("Save").pending(busy).build(&mut cx);
        let node = cx.get(id).expect("button");
        assert!(!node.a11y.is_focusable(), "pending button is not focusable");
    });
}
