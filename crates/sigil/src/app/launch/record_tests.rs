#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::cell::Cell;
use std::rc::Rc;

use crate::app::config::AppConfig;
use crate::app::embed::Host;
use crate::app::window::{CloseAction, allocate_window_id};
use crate::events::{Key, KeyEvent, KeyEventInit, KeyPhase, LogicalKey, ModifiersState};
use crate::events::{PhysicalKey, ShellEvent};
use crate::reactive::Runtime;
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

fn enter() -> ShellEvent {
    ShellEvent::Key(KeyEvent::new(KeyEventInit {
        key: LogicalKey::Named(Key::Enter),
        physical: PhysicalKey::Unidentified,
        modifiers: ModifiersState::empty(),
        repeat: false,
        phase: KeyPhase::Down,
        text: None,
    }))
}

fn deliver(windows: &mut Windows, handle: WinitWindowHandle) {
    let record = windows.get_mut(handle).expect("record");
    record.router.focus_next(record.host.cx());
    for event in std::mem::take(&mut record.view.inbox) {
        record.host.route(&mut record.router, event);
    }
}

#[test]
fn an_event_reaches_only_its_window_and_closing_one_keeps_the_other() {
    Runtime::scope("two-windows", || {
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
    Runtime::scope("close-to-tray", || {
        let mut windows = Windows::default();
        let config = AppConfig::new("tray").on_close(|| CloseAction::Hide);
        windows.insert(counting_window(7, Rc::new(Cell::new(0)), config));
        let handle = WinitWindowHandle::from_raw(7);
        assert_eq!(windows.close_requested(handle), CloseOutcome::Hidden);
        assert_eq!(windows.len(), 1);
        assert!(windows.get_mut(handle).expect("record").view.hidden);
    });
}
