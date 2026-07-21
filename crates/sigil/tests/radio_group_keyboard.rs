//! `radio_group` keyboard operation (5/5 Wave 3 T10): the group is
//! focusable and Arrow/Home/End move the selection with wraparound —
//! ARIA roving-selection behavior, selection follows movement.

#![allow(missing_docs)]
#![allow(unused_crate_dependencies)]
#![allow(clippy::unwrap_used)]
// Probe closures take a `&Cx` param named `cx` mirroring the outer test
// binding — the shadowing is intentional and local.
#![allow(clippy::shadow_unrelated)]

use arboard as _;
use criterion as _;
use image as _;
use pollster as _;
use sigil_actions as _;
use sigil_animation as _;
use sigil_animation_runtime as _;
use sigil_app as _;
use sigil_events as _;
use sigil_hal as _;
use sigil_kernel as _;
use sigil_layout as _;
use sigil_reactive as _;
use sigil_tasks as _;
use wgpu as _;

use sigil::prelude::*;
use sigil_hal::{KeyEventInit, PhysicalKey};

struct StubMeasure;
impl TextMeasure for StubMeasure {
    fn measure(&self, _content: &TextContent, _space: sigil_hal::Size) -> sigil_hal::LogicalSize {
        sigil_hal::LogicalSize::new(40.0, 16.0)
    }
}

fn compute(cx: &mut Cx, root: NodeId) -> LayoutEngine {
    let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
    layout
        .compute(cx, root, Size::fixed(320.0, 240.0))
        .expect("layout");
    layout
}

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
        let layout = compute(&mut cx, root);
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
        let _layout = compute(&mut cx, root);
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
