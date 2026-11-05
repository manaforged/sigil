#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use sigil::Host;
use sigil::app::FrameHooks;
use sigil::events::{Event, Key, KeyPhase, LogicalKey, ModifiersState, Shortcut};
use sigil::prelude::*;
use sigil::reactive::Owner;

const VIEW: (f32, f32) = (480.0, 320.0);

fn press(host: &mut Host, event: Event) {
    let _ = host.frame(
        VIEW,
        1.0 / 60.0,
        FrameHooks {
            input: Some(Box::new(move |target| {
                target.route(event);
                None
            })),
            ..FrameHooks::default()
        },
    );
    let _ = host.frame(VIEW, 1.0 / 60.0, FrameHooks::default());
}

#[test]
fn a_focused_terminal_gets_window_keys_except_command_shortcuts() {
    Owner::scope("terminal_keys", || {
        let typed = Rc::new(RefCell::new(String::new()));
        let (window_enter, window_command) = (Rc::new(Cell::new(0)), Rc::new(Cell::new(0)));
        let window_copy = Rc::new(Cell::new(0));
        let copy = Rc::clone(&window_copy);
        let mut host = Host::new(HostMode::Pixel);
        let (sink, enter, command) = (Rc::clone(&typed), Rc::clone(&window_enter), Rc::clone(&window_command));
        host.mount(
            column()
                .shortcut(Shortcut::new(LogicalKey::character("c"), ModifiersState::CONTROL), move |event| {
                    copy.set(copy.get() + 1);
                    event.stop_propagation();
                })
                .key_capture(Key::Enter, move |event| {
                    enter.set(enter.get() + 1);
                    event.stop_propagation();
                })
                .key_capture(Shortcut::new(LogicalKey::character("k"), ModifiersState::META), move |event| {
                    command.set(command.get() + 1);
                    event.stop_propagation();
                })
                .child(
                    sigil::widgets::terminal(signal(vec![String::new()]), move |bytes| sink.borrow_mut().push_str(&bytes))
                        .autofocus(),
                ),
        );
        for _ in 0..3 {
            let _ = host.frame(VIEW, 1.0 / 60.0, FrameHooks::default());
        }
        press(&mut host, Event::key(LogicalKey::character("a"), KeyPhase::Down).text(Some("a".into())));
        press(&mut host, Event::key(LogicalKey::Named(Key::Enter), KeyPhase::Down));
        press(&mut host, Event::key(LogicalKey::Named(Key::Tab), KeyPhase::Down));
        press(&mut host, Event::key(LogicalKey::character("c"), KeyPhase::Down).modifiers(ModifiersState::CONTROL));
        press(&mut host, Event::key(LogicalKey::character("k"), KeyPhase::Down).modifiers(ModifiersState::META));

        assert_eq!((window_enter.get(), window_copy.get()), (0, 0));
        assert_eq!(typed.borrow().as_str(), "a\r\t\u{3}");
        assert_eq!(window_command.get(), 1);
    });
}
