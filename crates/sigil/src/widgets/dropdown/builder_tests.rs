#![allow(clippy::unwrap_used)]

use super::dropdown;
use crate::Host;
use crate::events::Event;
use crate::prelude::*;
use crate::reactive::Owner;

#[test]
fn dropdown_selection_wins_over_controls_beneath_the_popup() {
    Owner::scope("dropdown-selection", || {
        let selected = signal(0);
        let changed = signal(0);
        let underneath = signal(0);
        let mut host = Host::new(HostMode::Pixel);
        host.mount(move || {
            container()
                .layout(LayoutStyle::flex_column().fill())
                .child(
                    dropdown(selected, ["First", "Second"]).change(move |value| changed.set(value)),
                )
                .child(button("Underneath").height(160.0).activate(move || {
                    underneath.set(underneath.peek() + 1);
                }))
        });
        host.layout((400.0, 300.0)).unwrap();
        click(&mut host, Point::new(20.0, 16.0));
        click_text(&mut host, "Second");
        assert_eq!(selected.peek(), 1);
        assert_eq!(changed.peek(), 1);
        assert_eq!(underneath.peek(), 0);
        click(&mut host, Point::new(20.0, 120.0));
        assert_eq!(underneath.peek(), 1);
        let combo = combo_box(&mut host);
        let (cx, _, router) = host.parts();
        router.set_focus(cx, combo);
        press_keys(
            &mut host,
            &[NamedKey::Enter, NamedKey::ArrowUp, NamedKey::Enter],
        );
        assert_eq!(selected.peek(), 0);
        assert_eq!(changed.peek(), 0);
    });
}

#[test]
fn clicking_a_scrollable_dropdown_commits_the_visible_option() {
    Owner::scope("scrollable-dropdown-selection", || {
        let selected = signal(0);
        let changed = signal(String::new());
        let options = ["Choose", "US", "Canada", "Mexico", "FR", "DE", "JP"];
        let mut host = Host::new(HostMode::Pixel);
        host.mount(move || {
            dropdown(selected, options.map(str::to_owned).to_vec())
                .change(move |index| changed.set(options.get(index).unwrap().to_string()))
        });
        host.layout((400.0, 300.0)).unwrap();
        click(&mut host, Point::new(20.0, 16.0));
        click_text(&mut host, "Canada");
        assert_eq!(
            changed.peek(),
            "Canada",
            "mouse selection must reach the caller"
        );
        let selected_scene = host.scene((400.0, 300.0)).unwrap();
        assert!(
            selected_scene.find_text("Canada").is_some(),
            "trigger shows the selected country"
        );
        assert!(
            selected_scene.find_text("Choose").is_none(),
            "the popup is closed"
        );
        press_keys(
            &mut host,
            &[NamedKey::Enter, NamedKey::ArrowDown, NamedKey::Enter],
        );
        assert_eq!(
            changed.peek(),
            "Mexico",
            "keyboard focus stays with the dropdown"
        );
    });
}

#[test]
fn escape_closes_the_menu_then_reaches_the_surrounding_dialog() {
    Owner::scope("dropdown-escape", || {
        let dismissed = signal(false);
        let mut host = Host::new(HostMode::Pixel);
        host.mount(move || {
            container()
                .key(NamedKey::Escape, move |_| dismissed.set(true))
                .child(dropdown(signal(0), ["First", "Second"]))
        });
        host.layout((400.0, 300.0)).unwrap();
        click(&mut host, Point::new(20.0, 16.0));
        press_keys(&mut host, &[NamedKey::Escape]);
        assert!(!dismissed.peek(), "first Escape only closes the open menu");
        assert!(
            host.scene((400.0, 300.0))
                .unwrap()
                .find_text("Second")
                .is_none()
        );
        press_keys(&mut host, &[NamedKey::Escape]);
        assert!(dismissed.peek(), "second Escape reaches the dialog");
    });
}

fn combo_box(host: &mut Host) -> Option<crate::kernel::NodeId> {
    let root = host.root().unwrap();
    let mut found = None;
    host.cx().visit_preorder(root, |id, node| {
        if node.a11y.role == Role::ComboBox {
            found = Some(id);
        }
    });
    found
}

fn press_keys(host: &mut Host, keys: &[NamedKey]) {
    for key in keys {
        host.route(Event::key(Key::Named(*key), KeyPhase::Down));
        host.layout((400.0, 300.0)).unwrap();
    }
}

fn click_text(host: &mut Host, label: &str) {
    let scene = host.scene((400.0, 300.0)).unwrap();
    let option = &scene.find_text(label).expect("visible option");
    click(
        host,
        Point::new(option.rect.origin.x + 5.0, option.rect.origin.y + 5.0),
    );
}

fn click(host: &mut Host, position: Point) {
    for phase in [PointerPhase::Move, PointerPhase::Down, PointerPhase::Up] {
        host.route(
            Event::pointer(phase, position)
                .button(Some(MouseButton::Left))
                .buttons(MouseButtons::LEFT),
        );
        host.layout((400.0, 300.0)).unwrap();
    }
}
