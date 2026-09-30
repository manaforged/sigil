#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::panic, clippy::unwrap_used)]

#[path = "events_support/input.rs"]
mod input;

use input::key_down;
use sigil::events::EventRouter;
use sigil::events::PhysicalKey;
use sigil::kernel::Cx;
use sigil::layout::LayoutEngine;
use sigil::prelude::*;
use sigil::widgets::virtual_list;
use sigil_kittest::stub_text_measure;

fn press(router: &mut EventRouter, cx: &mut Cx, layout: &LayoutEngine, key: Key) {
    let physical = match key {
        Key::ArrowDown => PhysicalKey::ArrowDown,
        Key::ArrowUp => PhysicalKey::ArrowUp,
        Key::PageDown => PhysicalKey::PageDown,
        Key::PageUp => PhysicalKey::PageUp,
        Key::Home => PhysicalKey::Home,
        Key::End => PhysicalKey::End,
        _ => PhysicalKey::Unidentified,
    };
    router.route(cx, layout, key_down(key, physical));
}

fn assert_px_eq(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() <= f32::EPSILON,
        "expected {expected}, got {actual}"
    );
}

#[test]
fn keyboard_navigation_moves_focus_and_scrolls_into_view() {
    sigil::reactive::Runtime::scope("sigil-virtual-list-keyboard", || {
        let focused_row = signal(0_usize);
        let scroll_y = signal(0.0_f32);
        let mut cx = Cx::new_root();
        let items: Vec<usize> = (0..100).collect();
        let root = cx.add(
            virtual_list(items, |_, _| container())
                .row_height(32.0)
                .viewport_height(480.0)
                .scroll_bind(scroll_y)
                .focused_row_bind(focused_row),
        );
        let mut layout = LayoutEngine::with_text_measure(stub_text_measure());
        layout
            .compute(&mut cx, root, Size::fixed(640.0, 480.0))
            .expect("layout");
        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(root));

        assert_eq!(focused_row.get(), 0);
        for _ in 0..20 {
            press(&mut router, &mut cx, &layout, Key::ArrowDown);
        }
        assert_eq!(focused_row.get(), 20);
        assert_px_eq(scroll_y.get(), 192.0);

        press(&mut router, &mut cx, &layout, Key::PageDown);
        assert_eq!(focused_row.get(), 35);
        assert_px_eq(scroll_y.get(), 672.0);

        press(&mut router, &mut cx, &layout, Key::End);
        assert_eq!(focused_row.get(), 99);
        assert_px_eq(scroll_y.get(), 2720.0);

        press(&mut router, &mut cx, &layout, Key::Home);
        assert_eq!(focused_row.get(), 0);
        assert_px_eq(scroll_y.get(), 0.0);
    });
}
