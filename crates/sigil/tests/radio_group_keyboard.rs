#![allow(clippy::unwrap_used)]
#![allow(clippy::shadow_unrelated)]

use crate::widget_support;

use sigil::events::EventRouter;
use sigil::events::PhysicalKey;
use sigil::events::{Event, KeyPhase};
use sigil::geometry::Constraint;
use sigil::kernel::Cx;
use sigil::layout::LayoutEngine;
use sigil::prelude::*;
use sigil::reactive::Owner;
use sigil::style::FocusRing;
use widget_support::compute;

const VIEWPORT: Constraint = Constraint::fixed(320.0, 240.0);

fn press(
    router: &mut EventRouter,
    cx: &mut Cx,
    layout: &LayoutEngine,
    key: NamedKey,
    physical: PhysicalKey,
) {
    router.route(
        cx,
        layout,
        Event::key(key, KeyPhase::Down).physical(physical),
    );
}

fn three_option_group(selected: Signal<usize>) -> impl IntoNode {
    radio_group(selected, ["Alpha", "Beta", "Gamma"])
}

#[test]
fn arrows_move_selection_with_wraparound() {
    Owner::scope("radio-kb-arrows", || {
        let selected = signal(0_usize);
        let mut cx = Cx::new_root();
        let root = cx.add(three_option_group(selected));
        let layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(root));

        let steps = [
            (
                NamedKey::ArrowDown,
                PhysicalKey::ArrowDown,
                1,
                "ArrowDown a → b",
            ),
            (
                NamedKey::ArrowDown,
                PhysicalKey::ArrowDown,
                2,
                "ArrowDown b → c",
            ),
            (
                NamedKey::ArrowDown,
                PhysicalKey::ArrowDown,
                0,
                "ArrowDown wraps c → a",
            ),
            (
                NamedKey::ArrowUp,
                PhysicalKey::ArrowUp,
                2,
                "ArrowUp wraps a → c",
            ),
            (NamedKey::Home, PhysicalKey::Home, 0, "Home selects first"),
            (NamedKey::End, PhysicalKey::End, 2, "End selects last"),
        ];
        for (key, physical, want, why) in steps {
            press(&mut router, &mut cx, &layout, key, physical);
            assert_eq!(selected.get(), want, "{why}");
        }
    });
}

#[test]
fn checked_row_shows_focus_ring_only_while_group_focused() {
    Owner::scope("radio-kb-ring", || {
        let selected = signal(1_usize);
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
                .and_then(|node| node.extension::<FocusRing>())
                .is_some_and(FocusRing::visible)
        };

        assert!(!ring_visible(&cx), "unfocused group shows no ring");
        router.set_focus(&mut cx, Some(root));
        assert!(ring_visible(&cx), "checked row rings while group focused");
        router.set_focus(&mut cx, None);
        assert!(!ring_visible(&cx), "ring clears on focus out");
    });
}
