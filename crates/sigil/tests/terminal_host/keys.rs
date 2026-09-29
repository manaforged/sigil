use super::*;

#[test]
fn map_enter_is_the_shell_enter_key() {
    use sigil::terminal::map_key;
    let event = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Enter,
        crossterm::event::KeyModifiers::NONE,
    );
    let mapped = map_key(&event).expect("enter maps");
    assert_eq!(mapped, key_down(Key::Enter));
}

#[test]
fn map_backtab_is_shift_tab() {
    use sigil::events::{Key, LogicalKey, ModifiersState, ShellEvent};
    use sigil::terminal::map_key;

    let event = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::BackTab,
        crossterm::event::KeyModifiers::NONE,
    );
    let Some(ShellEvent::Key(mapped)) = map_key(&event) else {
        panic!("backtab did not map");
    };
    assert_eq!(mapped.key, LogicalKey::Named(Key::Tab));
    assert_eq!(mapped.modifiers, ModifiersState::SHIFT);
}
#[test]
fn a_focus_change_reaches_the_terminal() {
    use sigil::Host;
    use sigil::events::{EventRouter, Key};

    sigil::reactive::Runtime::scope("tui-animate", || {
        let mut host = Host::cells();
        let mut router = EventRouter::new();
        host.mount(move || {
            container()
                .child(button("One").primary())
                .child(button("Two").primary())
        });
        host.layout((320.0, 64.0)).expect("layout");
        router.focus_next(host.cx());
        let first = step(&mut host, &mut router, [], 40, 8);
        let moved = step(&mut host, &mut router, [key_down(Key::Tab)], 40, 8);
        let settled = (0..12).fold(moved, |_, _| step(&mut host, &mut router, [], 40, 8));
        assert_ne!(
            first,
            settled,
            "moving focus must change what the terminal shows:\n{}",
            settled.text()
        );
    });
}

#[test]
fn typing_reaches_a_text_field() {
    use sigil::Host;
    use sigil::events::EventRouter;
    use sigil::reactive::signal;
    use sigil::terminal::map_key;
    use sigil::widgets::text_field;

    sigil::reactive::Runtime::scope("tui-typing", || {
        let value = signal(String::new());
        let mut host = Host::cells();
        let mut router = EventRouter::new();
        host.mount(move || text_field(value));
        drop(step(&mut host, &mut router, [], 40, 6));
        router.focus_next(host.cx());
        let typed = map_key(&crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char('x'),
            crossterm::event::KeyModifiers::NONE,
        ))
        .expect("char maps");
        let grid = step(&mut host, &mut router, [typed], 40, 6);
        assert!(
            grid.text().contains('x'),
            "expected typed x in:\n{}",
            grid.text()
        );
        assert_eq!(value.peek(), "x");
    });
}

#[test]
fn a_control_chord_inserts_nothing() {
    use sigil::Host;
    use sigil::events::EventRouter;
    use sigil::reactive::signal;
    use sigil::terminal::map_key;
    use sigil::widgets::text_field;

    sigil::reactive::Runtime::scope("tui-chord", || {
        let value = signal(String::new());
        let mut host = Host::cells();
        let mut router = EventRouter::new();
        host.mount(move || text_field(value));
        drop(step(&mut host, &mut router, [], 40, 6));
        router.focus_next(host.cx());
        let chord = map_key(&crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char('x'),
            crossterm::event::KeyModifiers::CONTROL,
        ))
        .expect("char maps");
        drop(step(&mut host, &mut router, [chord], 40, 6));
        assert_eq!(value.peek(), "");
    });
}

#[test]
fn paste_inserts_whole() {
    use sigil::Host;
    use sigil::events::EventRouter;
    use sigil::reactive::signal;
    use sigil::terminal::paste;
    use sigil::widgets::text_field;

    sigil::reactive::Runtime::scope("tui-paste", || {
        let value = signal(String::new());
        let mut host = Host::cells();
        let mut router = EventRouter::new();
        host.mount(move || text_field(value));
        drop(step(&mut host, &mut router, [], 40, 6));
        router.focus_next(host.cx());
        drop(step(&mut host, &mut router, [paste("hello world")], 40, 6));
        assert_eq!(value.peek(), "hello world");
    });
}
