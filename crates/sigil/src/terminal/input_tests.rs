use std::cell::RefCell;
use std::rc::Rc;

use crate::events::{Event, PointerPhase};
use crate::prelude::*;
use crate::terminal::Screen;

const SIZE: (u16, u16) = (40, 12);

fn exits(host: &mut crate::Host, events: impl IntoIterator<Item = Event>) -> bool {
    let mut screen = Screen::new(SIZE.0, SIZE.1);
    screen.frame(host, events, SIZE).is_ok() && screen.exited()
}

fn pointer(phase: PointerPhase, col: u16) -> Event {
    let at = crate::terminal::cell_point(col, 0);
    match phase {
        PointerPhase::Down => Event::pointer_down(at),
        PointerPhase::Up => Event::pointer_up(at),
        _ => Event::pointer_drag(at),
    }
}

#[test]
fn escape_closes_an_open_modal_before_it_exits() {
    crate::reactive::Owner::scope("quit-escape-modal", || {
        let show = signal(true);
        let mut host = crate::Host::new(HostMode::Cell);
        host.mount(move || container().child(modal(show).child(button("Close"))));
        host.set_quit(AppConfig::new("quit").exit_on_esc(true).quit);
        assert!(!exits(&mut host, []), "the first frame exited");

        let closed = !exits(&mut host, [Event::key_down(NamedKey::Escape)]);
        assert!(closed, "Escape exited the app while a modal was open");
        assert!(!show.peek(), "Escape left the modal open");

        let exited = exits(&mut host, [Event::key_down(NamedKey::Escape)]);
        assert!(exited, "Escape with nothing open did not exit the app");
    });
}

#[test]
fn ctrl_c_copies_a_selection_before_it_exits() {
    crate::reactive::Owner::scope("quit-ctrl-c-copy", || {
        let copied = Rc::new(RefCell::new(Vec::new()));
        let seen = Rc::clone(&copied);
        let mut host = crate::Host::new(HostMode::Cell);
        host.mount(move || {
            let seen = Rc::clone(&seen);
            container()
                .selectable_text(true)
                .selection_copy(move |text, _| seen.borrow_mut().push(text.to_owned()))
                .child(text("The quick brown fox"))
        });
        host.set_quit(AppConfig::new("quit").quit);
        assert!(!exits(&mut host, []), "the first frame exited");
        for (phase, col) in [
            (PointerPhase::Move, 4),
            (PointerPhase::Down, 4),
            (PointerPhase::Move, 6),
            (PointerPhase::Move, 8),
            (PointerPhase::Up, 8),
        ] {
            assert!(!exits(&mut host, [pointer(phase, col)]), "a drag exited");
        }
        copied.borrow_mut().clear();
        let ctrl_c = || Event::key_down(Key::character("c")).modifiers(Modifiers::CTRL);

        let kept = !exits(&mut host, [ctrl_c()]);
        assert!(kept, "Ctrl+C exited the app while text was selected");
        assert_eq!(copied.borrow().as_slice(), &["quick".to_owned()]);

        assert!(!exits(&mut host, [Event::key_down(NamedKey::Escape)]));
        assert!(
            exits(&mut host, [ctrl_c()]),
            "Ctrl+C with nothing selected did not exit"
        );
    });
}

#[test]
fn a_closed_peer_reads_as_a_hang_up() -> std::io::Result<()> {
    let (ours, theirs) = std::os::unix::net::UnixStream::pair()?;
    assert!(!super::hung_up(&ours));
    drop(theirs);
    assert!(super::hung_up(&ours));
    Ok(())
}

#[test]
fn a_hang_up_during_the_wait_ends_it() -> std::io::Result<()> {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::Duration;
    let hup = std::sync::Arc::new(AtomicBool::new(false));
    signal_hook::flag::register(signal_hook::consts::SIGHUP, std::sync::Arc::clone(&hup))?;
    let (rx, _tx) = std::os::unix::net::UnixStream::pair()?;
    rx.set_nonblocking(true)?;
    let wake = super::InputWake { rx: Some(rx) };
    let (ours, theirs) = std::os::unix::net::UnixStream::pair()?;
    let closer = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(100));
        drop(theirs);
    });
    let wait = Duration::from_secs(5);
    let ready = wake.wait(&ours, (Some(wait), wait), true, |_| Ok(false))?;
    drop(closer.join());
    assert!(!ready);
    assert!(hup.load(Ordering::SeqCst), "no SIGHUP raised");
    Ok(())
}

#[test]
fn a_hang_up_while_not_listening_ends_the_wait_without_the_event_reader() -> std::io::Result<()> {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::{Duration, Instant};
    let hup = std::sync::Arc::new(AtomicBool::new(false));
    signal_hook::flag::register(signal_hook::consts::SIGHUP, std::sync::Arc::clone(&hup))?;
    let wake = super::InputWake { rx: None };
    let (ours, theirs) = std::os::unix::net::UnixStream::pair()?;
    let closer = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(100));
        drop(theirs);
    });
    let wait = Duration::from_secs(5);
    let started = Instant::now();
    let blind_reader = |limit: Duration| {
        std::thread::sleep(limit);
        Ok(false)
    };
    let ready = wake.wait(&ours, (Some(wait), wait), false, blind_reader)?;
    drop(closer.join());
    assert!(!ready);
    assert!(hup.load(Ordering::SeqCst), "no SIGHUP raised");
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "the wait ran to its limit"
    );
    Ok(())
}

#[test]
fn an_event_reader_at_end_of_file_reads_as_a_hang_up() -> std::io::Result<()> {
    use std::sync::atomic::{AtomicBool, Ordering};
    let hup = std::sync::Arc::new(AtomicBool::new(false));
    signal_hook::flag::register(signal_hook::consts::SIGHUP, std::sync::Arc::clone(&hup))?;
    let eof = || Err(std::io::Error::from(std::io::ErrorKind::UnexpectedEof));
    assert!(!super::ended(eof())?);
    assert!(hup.load(Ordering::SeqCst), "no SIGHUP raised");
    Ok(())
}

#[test]
fn ctrl_c_exits_with_an_empty_field_focused() {
    crate::reactive::Owner::scope("quit-ctrl-c-field", || {
        let value = signal(String::new());
        let mut host = crate::Host::new(HostMode::Cell);
        host.mount(move || container().child(text_field(value).autofocus()));
        host.set_quit(AppConfig::new("quit").quit);
        assert!(!exits(&mut host, []), "the first frame exited");
        let ctrl_c = Event::key_down(Key::character("c")).modifiers(Modifiers::CTRL);
        assert!(
            exits(&mut host, [ctrl_c]),
            "Ctrl+C with an empty field focused did not exit"
        );
    });
}
