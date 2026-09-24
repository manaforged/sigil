#![allow(clippy::unwrap_used)]

use super::dropdown;
use crate::hal::KeyEventInit;
use crate::prelude::*;
use crate::reactive::Runtime;

#[test]
fn dropdown_selection_wins_over_controls_beneath_the_popup() {
    Runtime::scope("dropdown-selection", || {
        let selected = signal(0);
        let changed = signal(0);
        let underneath = signal(0);
        let mut host = Host::new();
        host.build(|cx| {
            cx.add(
                container()
                    .layout(LayoutStyle::flex_column().fill())
                    .child(
                        dropdown(selected, vec!["First".into(), "Second".into()])
                            .change(move |value| changed.set(value)),
                    )
                    .child(button("Underneath").height(160.0).activate(move || {
                        underneath.set(underneath.peek() + 1);
                    })),
            )
        });
        host.layout((400.0, 300.0)).unwrap();
        let mut router = EventRouter::new();
        click(&mut host, &mut router, Point::new(20.0, 16.0));
        host.layout((400.0, 300.0)).unwrap();
        let scene = host.scene((400.0, 300.0)).unwrap();
        let option = &scene.find_text("Second").expect("open option").run;
        click(
            &mut host,
            &mut router,
            Point::new(option.rect.origin.x + 5.0, option.rect.origin.y + 5.0),
        );
        host.layout((400.0, 300.0)).unwrap();
        assert_eq!(selected.peek(), 1);
        assert_eq!(changed.peek(), 1);
        assert_eq!(underneath.peek(), 0);
        click(&mut host, &mut router, Point::new(20.0, 120.0));
        assert_eq!(underneath.peek(), 1);
        let combo = combo_box(&mut host);
        router.set_focus(host.cx(), combo);
        press_keys(
            &mut host,
            &mut router,
            &[Key::Enter, Key::ArrowUp, Key::Enter],
        );
        assert_eq!(selected.peek(), 0);
        assert_eq!(changed.peek(), 0);
    });
}

fn combo_box(host: &mut Host) -> Option<crate::kernel::NodeId> {
    let root = host.root().unwrap();
    let mut found = None;
    host.cx().visit_preorder(root, |id, node| {
        if node.a11y.role == Role::ComboBox {
            found = Some(id);
        }
    });
    found
}

fn press_keys(host: &mut Host, router: &mut EventRouter, keys: &[Key]) {
    for key in keys {
        host.route(
            router,
            ShellEvent::Key(KeyEvent::new(KeyEventInit {
                key: LogicalKey::Named(*key),
                physical: PhysicalKey::Unidentified,
                modifiers: ModifiersState::empty(),
                repeat: false,
                phase: KeyPhase::Down,
                text: None,
            })),
        );
        host.layout((400.0, 300.0)).unwrap();
    }
}

fn click(host: &mut Host, router: &mut EventRouter, position: Point) {
    for phase in [PointerPhase::Move, PointerPhase::Down, PointerPhase::Up] {
        host.route(
            router,
            ShellEvent::Pointer(PointerEvent::new(PointerEventInit {
                position,
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
