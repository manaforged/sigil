use super::*;
use sigil::kernel::HostMode;

#[test]
fn map_enter_is_the_shell_enter_key() {
    use sigil::terminal::map_key;
    let event = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Enter,
        crossterm::event::KeyModifiers::NONE,
    );
    let mapped = map_key(&event).expect("enter maps");
    assert_eq!(mapped, Event::key_down(Key::Enter));
}

#[test]
fn map_backtab_is_shift_tab() {
    use sigil::events::{Event, Key, LogicalKey, ModifiersState};
    use sigil::terminal::map_key;

    let event = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::BackTab,
        crossterm::event::KeyModifiers::NONE,
    );
    let Some(Event::Key(mapped)) = map_key(&event) else {
        panic!("backtab did not map");
    };
    assert_eq!(mapped.key, LogicalKey::Named(Key::Tab));
    assert_eq!(mapped.modifiers, ModifiersState::SHIFT);
}
#[test]
fn a_focus_change_reaches_the_terminal() {
    use sigil::Host;
    use sigil::events::Key;

    sigil::reactive::Owner::scope("tui-animate", || {
        let mut host = Host::new(HostMode::Cell);
        host.mount(move || {
            container()
                .child(button("One").primary())
                .child(button("Two").primary())
        });
        host.layout((320.0, 64.0)).expect("layout");
        let (cx, layout, router) = host.parts();
        router.focus_next(cx, layout);
        let first = step(&mut host, [], 40, 8);
        let moved = step(&mut host, [Event::key_down(Key::Tab)], 40, 8);
        let settled = (0..12).fold(moved, |_, _| step(&mut host, [], 40, 8));
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
    use sigil::reactive::signal;
    use sigil::terminal::map_key;
    use sigil::widgets::text_field;

    sigil::reactive::Owner::scope("tui-typing", || {
        let value = signal(String::new());
        let mut host = Host::new(HostMode::Cell);
        host.mount(move || text_field(value));
        drop(step(&mut host, [], 40, 6));
        let (cx, layout, router) = host.parts();
        router.focus_next(cx, layout);
        let typed = map_key(&crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char('x'),
            crossterm::event::KeyModifiers::NONE,
        ))
        .expect("char maps");
        let grid = step(&mut host, [typed], 40, 6);
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
    use sigil::reactive::signal;
    use sigil::terminal::map_key;
    use sigil::widgets::text_field;

    sigil::reactive::Owner::scope("tui-chord", || {
        let value = signal(String::new());
        let mut host = Host::new(HostMode::Cell);
        host.mount(move || text_field(value));
        drop(step(&mut host, [], 40, 6));
        let (cx, layout, router) = host.parts();
        router.focus_next(cx, layout);
        let chord = map_key(&crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char('x'),
            crossterm::event::KeyModifiers::CONTROL,
        ))
        .expect("char maps");
        drop(step(&mut host, [chord], 40, 6));
        assert_eq!(value.peek(), "");
    });
}

#[test]
fn paste_inserts_whole() {
    use sigil::Host;
    use sigil::reactive::signal;
    use sigil::widgets::text_field;

    sigil::reactive::Owner::scope("tui-paste", || {
        let value = signal(String::new());
        let mut host = Host::new(HostMode::Cell);
        host.mount(move || text_field(value));
        drop(step(&mut host, [], 40, 6));
        let (cx, layout, router) = host.parts();
        router.focus_next(cx, layout);
        drop(step(&mut host, [Event::paste("hello world")], 40, 6));
        assert_eq!(value.peek(), "hello world");
    });
}

#[test]
fn a_paste_reaches_the_focused_node_as_one_event() {
    use std::cell::RefCell;
    use std::rc::Rc;

    use sigil::Host;
    use sigil::prelude::Widget;

    sigil::reactive::Owner::scope("tui-paste-event", || {
        let seen = Rc::new(RefCell::new(Vec::new()));
        let record = Rc::clone(&seen);
        let mut host = Host::new(HostMode::Cell);
        host.mount(move || {
            let record = Rc::clone(&record);
            button("Target").on_paste(move |text| {
                record.borrow_mut().push(text.to_owned());
                true
            })
        });
        drop(step(&mut host, [], 40, 6));
        let (cx, layout, router) = host.parts();
        router.focus_next(cx, layout);
        drop(step(&mut host, [Event::paste("one\ntwo")], 40, 6));
        assert_eq!(*seen.borrow(), vec!["one\ntwo".to_owned()]);
    });
}

#[test]
fn an_autofocus_field_in_a_modal_takes_keys_when_the_modal_opens() {
    use sigil::Host;
    use sigil::reactive::signal;
    use sigil::widgets::{modal, text_field};

    sigil::reactive::Owner::scope("tui-modal-autofocus", || {
        let open = signal(false);
        let query = signal(String::new());
        let mut host = Host::new(HostMode::Cell);
        host.mount(move || {
            container().child(
                modal(open)
                    .title("Palette")
                    .child(text_field(query).autofocus()),
            )
        });
        drop(step(&mut host, [], 40, 12));
        open.set(true);
        drop(step(&mut host, [], 40, 12));
        let typed = sigil::terminal::map_key(&crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char('d'),
            crossterm::event::KeyModifiers::NONE,
        ))
        .expect("char maps");
        drop(step(&mut host, [typed], 40, 12));
        assert_eq!(query.peek(), "d");
    });
}
