#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::cell::RefCell;
use std::rc::Rc;

use sigil::ErrorKind;
use sigil::events::{Event, Gesture, GestureEvent, Key, Modifiers, ScrollPhase, Shortcut};
use sigil::layout::LayoutStyle;
use sigil::prelude::*;
use sigil::reactive::Owner;

#[test]
fn a_shortcut_parses_from_both_label_forms() {
    let expected = Shortcut::new(Key::character("k"), Modifiers::CTRL | Modifiers::SHIFT);
    assert_eq!("ctrl+shift+k".parse::<Shortcut>(), Ok(expected.clone()));
    assert_eq!(expected.label_for(false).parse::<Shortcut>(), Ok(expected));
    let mac = Shortcut::new(Key::character("k"), Modifiers::META | Modifiers::SHIFT);
    assert_eq!(mac.label_for(true).parse::<Shortcut>(), Ok(mac));
    assert!(matches!(
        "hyper+k".parse::<Shortcut>(),
        Err(error) if error.kind() == ErrorKind::UnknownModifier
    ));
}

#[test]
fn a_gesture_reaches_the_node_under_it() {
    Owner::scope("gesture-route", || {
        let seen = Rc::new(RefCell::new(Vec::new()));
        let record = Rc::clone(&seen);
        let mut host = sigil::Host::new(sigil::kernel::HostMode::Cell);
        host.mount(move || {
            let record = Rc::clone(&record);
            container().layout(LayoutStyle::block().fill()).gesture(
                move |event: &mut GestureEvent| {
                    record.borrow_mut().push(event.kind());
                },
            )
        });
        host.grid([], 20, 6).unwrap();
        let pinch = Event::gesture(
            Gesture::Pinch(0.25),
            ScrollPhase::Moved,
            sigil::terminal::cell_point(4, 2),
        );
        host.grid([pinch], 20, 6).unwrap();
        assert_eq!(*seen.borrow(), vec![Gesture::Pinch(0.25)]);
    });
}
