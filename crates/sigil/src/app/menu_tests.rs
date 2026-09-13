use std::cell::Cell;
use std::rc::Rc;

use crate::reactive::Runtime;

use super::*;

#[test]
fn events_channel_delivers_activations_to_subscribers() {
    Runtime::scope("menu-events", || {
        let hits: Rc<Cell<u32>> = Rc::new(Cell::new(0));
        let subscription = events().subscribe({
            let hits = Rc::clone(&hits);
            move |id| {
                assert_eq!(id, "open");
                hits.set(hits.get() + 1);
            }
        });
        simulate(MenuId::new("open"));
        assert_eq!(hits.get(), 1, "subscriber must fire on a simulated click");
        drop(subscription);
    });
}

#[cfg(not(target_os = "macos"))]
#[test]
fn set_menu_bar_after_install_queues_again() {
    set_menu_bar(Menu::new());
    assert!(has_pending());
    PENDING.with(|slot| *slot.borrow_mut() = None);
    INSTALLED.with(|slot| *slot.borrow_mut() = Some(Menu::new()));
    set_menu_bar(Menu::new());
    assert!(has_pending());
    PENDING.with(|slot| *slot.borrow_mut() = None);
    INSTALLED.with(|slot| *slot.borrow_mut() = None);
}
