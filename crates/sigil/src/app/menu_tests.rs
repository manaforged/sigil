use std::cell::Cell;
use std::rc::Rc;

use crate::reactive::Owner;

use super::*;

#[test]
fn events_channel_delivers_activations_to_subscribers() {
    Owner::scope("menu-events", || {
        let hits: Rc<Cell<u32>> = Rc::new(Cell::new(0));
        let subscription = Menu::events().subscribe({
            let hits = Rc::clone(&hits);
            move |id| {
                assert_eq!(id, "open");
                hits.set(hits.get() + 1);
            }
        });
        simulate("open");
        assert_eq!(hits.get(), 1, "subscriber must fire on a simulated click");
        drop(subscription);
    });
}

#[test]
fn install_reports_a_platform_with_no_menu_bar() {
    assert_eq!(Menu::new().install().is_ok(), NATIVE_MENU_BAR);
    assert_eq!(has_pending(), NATIVE_MENU_BAR);
    PENDING.with(|slot| *slot.borrow_mut() = None);
}

#[cfg(target_os = "windows")]
#[test]
fn install_after_install_queues_again() {
    Menu::new().install().expect("install");
    assert!(has_pending());
    PENDING.with(|slot| *slot.borrow_mut() = None);
    INSTALLED.with(|slot| *slot.borrow_mut() = Some(muda::Menu::new()));
    Menu::new().install().expect("install");
    assert!(has_pending());
    PENDING.with(|slot| *slot.borrow_mut() = None);
    INSTALLED.with(|slot| *slot.borrow_mut() = None);
}

#[test]
fn accelerator_rejects_a_key_it_cannot_map() {
    let save = Shortcut::primary(LogicalKey::character("s"));
    assert!(accelerator(&save).is_ok());
    for key in [
        Key::F13,
        Key::F14,
        Key::F15,
        Key::F16,
        Key::F17,
        Key::F18,
        Key::F19,
        Key::F20,
        Key::F21,
        Key::F22,
        Key::F23,
        Key::F24,
    ] {
        assert!(accelerator(&Shortcut::plain(key)).is_ok(), "{key} maps");
    }
    assert!(accelerator(&Shortcut::plain(Key::F25)).is_err());
}

#[test]
fn a_shortcut_with_no_native_key_drops_only_the_accelerator() {
    let f25 = Shortcut::plain(Key::F25);
    assert!(item_accelerator(Some(&f25)).is_none());
    assert!(item_accelerator(Some(&Shortcut::plain(Key::F13))).is_some());
    assert!(item_accelerator(None).is_none());
}

#[cfg(not(target_os = "macos"))]
#[test]
fn a_shortcut_with_no_native_key_keeps_its_item() {
    let menu = Menu::new()
        .item("open", "Open", Some(Shortcut::plain(Key::F25)))
        .check("wrap", "Wrap", true, Some(Shortcut::plain(Key::F25)))
        .native()
        .expect("native menu");
    assert_eq!(menu.items().len(), 2);
}
