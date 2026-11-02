#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::cell::Cell;
use std::rc::Rc;

use crate::app::config::AppConfig;
use crate::app::embed::Host;
use crate::app::window::{CloseAction, allocate_window_id};
use crate::events::Event;
use crate::events::{Key, KeyPhase, LogicalKey};
use crate::reactive::Owner;
use crate::shell::WinitWindowHandle;
use crate::widgets::button;

use super::{CloseOutcome, Routed, WindowRecord, Windows};

fn counting_window(n: u64, count: Rc<Cell<u32>>, config: AppConfig) -> WindowRecord {
    WindowRecord::new(
        allocate_window_id(),
        WinitWindowHandle::from_raw(n),
        config,
        Box::new(move |host: &mut Host| {
            host.mount(move || {
                let count = Rc::clone(&count);
                button("Count").activate(move || count.set(count.get() + 1))
            });
        }),
    )
    .expect("window record")
}

fn enter() -> Event {
    Event::key(LogicalKey::Named(Key::Enter), KeyPhase::Down)
}

fn deliver(windows: &mut Windows, handle: WinitWindowHandle) {
    let record = windows.get_mut(handle).expect("record");
    record.host.layout((200.0, 100.0)).expect("layout");
    let (cx, layout, router) = record.host.parts();
    router.focus_next(cx, layout);
    for event in std::mem::take(&mut record.view.inbox) {
        record.host.route(event);
    }
}

#[test]
fn an_event_reaches_only_its_window_and_closing_one_keeps_the_other() {
    Owner::scope("two-windows", || {
        let (first, second) = (Rc::new(Cell::new(0)), Rc::new(Cell::new(0)));
        let mut windows = Windows::default();
        windows.insert(counting_window(0, Rc::clone(&first), AppConfig::new("a")));
        windows.insert(counting_window(1, Rc::clone(&second), AppConfig::new("b")));
        let target = WinitWindowHandle::from_raw(1);
        assert!(matches!(windows.route(target, enter()), Routed::Delivered));
        deliver(&mut windows, WinitWindowHandle::from_raw(0));
        deliver(&mut windows, target);
        assert_eq!((first.get(), second.get()), (0, 1));
        assert_eq!(
            windows.close_requested(target),
            CloseOutcome::Removed { last: false }
        );
        assert_eq!(windows.len(), 1);
    });
}

#[test]
fn close_to_tray_hides_the_window_and_keeps_it() {
    Owner::scope("close-to-tray", || {
        let mut windows = Windows::default();
        let config = AppConfig::new("tray").on_close(|| CloseAction::Hide);
        windows.insert(counting_window(7, Rc::new(Cell::new(0)), config));
        let handle = WinitWindowHandle::from_raw(7);
        assert_eq!(windows.close_requested(handle), CloseOutcome::Hidden);
        assert_eq!(windows.len(), 1);
        assert!(windows.get_mut(handle).expect("record").view.hidden);
    });
}
