#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used)]
#![allow(clippy::shadow_unrelated)]

#[path = "widget_support/mod.rs"]
mod widget_support;

use sigil::hal::{KeyEventInit, PhysicalKey};
use sigil::prelude::*;
use widget_support::compute;

const VIEWPORT: Size = Size::fixed(320.0, 240.0);

fn press(
    router: &mut EventRouter,
    cx: &mut Cx,
    layout: &LayoutEngine,
    key: Key,
    physical: PhysicalKey,
) {
    router.route(
        cx,
        layout,
        ShellEvent::Key(KeyEvent::new(KeyEventInit {
            key: key.into(),
            physical,
            modifiers: ModifiersState::empty(),
            repeat: false,
            phase: KeyPhase::Down,
            text: None,
        })),
    );
}

fn three_option_group(selected: Signal<String>) -> RadioGroupBuilder {
    radio_group(selected)
        .option("a", "Alpha")
        .option("b", "Beta")
        .option("c", "Gamma")
}

#[test]
fn arrows_move_selection_with_wraparound() {
    Runtime::scope("radio-kb-arrows", || {
        let selected = signal(String::from("a"));
        let mut cx = Cx::new_root();
        let root = cx.add(three_option_group(selected));
        let layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(root));

        press(
            &mut router,
            &mut cx,
            &layout,
            Key::ArrowDown,
            PhysicalKey::ArrowDown,
        );
        assert_eq!(selected.get(), "b", "ArrowDown a → b");
        press(
            &mut router,
            &mut cx,
            &layout,
            Key::ArrowDown,
            PhysicalKey::ArrowDown,
        );
        assert_eq!(selected.get(), "c", "ArrowDown b → c");
        press(
            &mut router,
            &mut cx,
            &layout,
            Key::ArrowDown,
            PhysicalKey::ArrowDown,
        );
        assert_eq!(selected.get(), "a", "ArrowDown wraps c → a");
        press(
            &mut router,
            &mut cx,
            &layout,
            Key::ArrowUp,
            PhysicalKey::ArrowUp,
        );
        assert_eq!(selected.get(), "c", "ArrowUp wraps a → c");
        press(&mut router, &mut cx, &layout, Key::Home, PhysicalKey::Home);
        assert_eq!(selected.get(), "a", "Home selects first");
        press(&mut router, &mut cx, &layout, Key::End, PhysicalKey::End);
        assert_eq!(selected.get(), "c", "End selects last");
    });
}

#[test]
fn checked_row_shows_focus_ring_only_while_group_focused() {
    Runtime::scope("radio-kb-ring", || {
        let selected = signal(String::from("b"));
        let mut cx = Cx::new_root();
        let root = cx.add(three_option_group(selected));
        let _layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
        let mut router = EventRouter::new();

        let checked_row = cx
            .get(root)
            .expect("group")
            .children()
            .get(1)
            .copied()
            .expect("second row");
        let ring_visible = |cx: &Cx| {
            cx.get(checked_row)
                .and_then(|node| node.get_extension::<FocusRing>())
                .is_some_and(FocusRing::visible)
        };

        assert!(!ring_visible(&cx), "unfocused group shows no ring");
        router.set_focus(&mut cx, Some(root));
        assert!(ring_visible(&cx), "checked row rings while group focused");
        router.set_focus(&mut cx, None);
        assert!(!ring_visible(&cx), "ring clears on focus out");
    });
}
