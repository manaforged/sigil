#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::cell::RefCell;
use std::rc::Rc;

use sigil::events::{
    EventRouter, GestureEvent, GestureEventInit, GestureKind, LogicalKey, ModifiersState,
    ScrollPhase, ShellEvent, Shortcut, ShortcutParseError,
};
use sigil::prelude::*;
use sigil::reactive::Runtime;

#[test]
fn a_shortcut_parses_from_both_label_forms() {
    let expected = Shortcut::new(
        LogicalKey::character("k"),
        ModifiersState::CONTROL | ModifiersState::SHIFT,
    );
    assert_eq!("ctrl+shift+k".parse::<Shortcut>(), Ok(expected.clone()));
    assert_eq!(expected.label_for(false).parse::<Shortcut>(), Ok(expected));
    let mac = Shortcut::new(
        LogicalKey::character("k"),
        ModifiersState::META | ModifiersState::SHIFT,
    );
    assert_eq!(mac.label_for(true).parse::<Shortcut>(), Ok(mac));
    assert!(matches!(
        "hyper+k".parse::<Shortcut>(),
        Err(ShortcutParseError::UnknownModifier(_))
    ));
}

#[test]
fn a_gesture_reaches_the_node_under_it() {
    Runtime::scope("gesture-route", || {
        let seen = Rc::new(RefCell::new(Vec::new()));
        let record = Rc::clone(&seen);
        let mut host = sigil::Host::cells();
        host.mount(move || {
            let record = Rc::clone(&record);
            container().layout(LayoutStyle::block().fill()).on_gesture(
                move |event: &mut GestureEvent| {
                    record.borrow_mut().push(event.kind);
                },
            )
        });
        let mut router = EventRouter::new();
        sigil::terminal::step(&mut host, &mut router, [], 20, 6).unwrap();
        let pinch = ShellEvent::Gesture(GestureEvent::new(GestureEventInit {
            kind: GestureKind::Pinch(0.25),
            phase: ScrollPhase::Moved,
            position: sigil::terminal::cell_point(4, 2),
            modifiers: ModifiersState::empty(),
        }));
        sigil::terminal::step(&mut host, &mut router, [pinch], 20, 6).unwrap();
        assert_eq!(*seen.borrow(), vec![GestureKind::Pinch(0.25)]);
    });
}
