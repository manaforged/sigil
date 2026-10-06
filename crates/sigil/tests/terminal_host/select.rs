use std::cell::RefCell;
use std::rc::Rc;

use sigil::clipboard::{ClipError, Clipboard, Delivery};
use sigil::events::{EventRouter, PointerPhase};
use sigil::prelude::NodeEventsExt;
use sigil::terminal::pointer;

use super::*;

struct Recorder(Rc<RefCell<Vec<String>>>);

impl Clipboard for Recorder {
    fn set(&self, text: &str) -> Result<Delivery, ClipError> {
        self.0.borrow_mut().push(text.to_owned());
        Ok(Delivery::Confirmed)
    }

    fn get(&self) -> Option<String> {
        self.0.borrow().last().cloned()
    }
}

fn drag_copies(from: u16, to: u16) -> Option<String> {
    let copied = Rc::new(RefCell::new(Vec::new()));
    sigil::clipboard::install(Recorder(Rc::clone(&copied)));
    let mut host = sigil::Host::cells();
    host.mount(|| {
        container()
            .selectable_text(true)
            .child(text("The quick brown fox"))
    });
    let mut router = EventRouter::new();
    step(&mut host, &mut router, [], 40, 4);
    for phase in [PointerPhase::Move, PointerPhase::Down] {
        step(&mut host, &mut router, [pointer(phase, from, 0)], 40, 4);
    }
    let middle = u16::midpoint(from, to);
    for col in [middle, to] {
        step(
            &mut host,
            &mut router,
            [pointer(PointerPhase::Move, col, 0)],
            40,
            4,
        );
    }
    step(
        &mut host,
        &mut router,
        [pointer(PointerPhase::Up, to, 0)],
        40,
        4,
    );
    copied.borrow().last().cloned()
}

#[test]
fn a_forward_drag_copies_the_cell_it_started_on() {
    assert_eq!(drag_copies(0, 18).as_deref(), Some("The quick brown fox"));
}

#[test]
fn a_backward_drag_copies_the_cell_it_started_on() {
    assert_eq!(drag_copies(18, 0).as_deref(), Some("The quick brown fox"));
}

#[test]
fn a_drag_inside_a_line_copies_exactly_the_cells_it_crossed() {
    assert_eq!(drag_copies(4, 8).as_deref(), Some("quick"));
}

#[test]
fn a_drag_copy_reports_what_it_copied() {
    let copied = Rc::new(RefCell::new(Vec::new()));
    sigil::clipboard::install(Recorder(Rc::new(RefCell::new(Vec::new()))));
    let seen = Rc::clone(&copied);
    let mut host = sigil::Host::cells();
    host.mount(move || {
        let seen = Rc::clone(&seen);
        container()
            .selectable_text(true)
            .on_selection_copy(move |text, result| {
                seen.borrow_mut().push((text.to_owned(), result.is_ok()));
            })
            .child(text("The quick brown fox"))
    });
    let mut router = EventRouter::new();
    step(&mut host, &mut router, [], 40, 4);
    for (phase, col) in [
        (PointerPhase::Move, 4),
        (PointerPhase::Down, 4),
        (PointerPhase::Move, 6),
        (PointerPhase::Move, 8),
        (PointerPhase::Up, 8),
    ] {
        step(&mut host, &mut router, [pointer(phase, col, 0)], 40, 4);
    }
    assert_eq!(copied.borrow().as_slice(), &[("quick".to_owned(), true)]);
}
