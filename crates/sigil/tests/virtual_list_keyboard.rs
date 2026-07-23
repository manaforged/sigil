#![allow(missing_docs)]
#![allow(clippy::panic, clippy::unwrap_used)]

use arboard as _;
use criterion as _;
use image as _;
use pollster as _;
use sigil_actions as _;
use sigil_animation as _;
use sigil_animation_runtime as _;
use sigil_app as _;
use sigil_bidi as _;
use sigil_events as _;
use sigil_hal as _;
use sigil_kernel as _;
use sigil_layout as _;
use sigil_reactive as _;
use sigil_renderer as _;
use sigil_style as _;
use sigil_tasks as _;
use tiny_skia as _;
use wgpu as _;

use sigil::prelude::*;
use sigil::widgets::virtual_list;
use sigil_hal::{KeyEventInit, PhysicalKey};

struct StubMeasure;

impl TextMeasure for StubMeasure {
    fn measure(&self, _content: &TextContent, _space: sigil_hal::Size) -> sigil_hal::LogicalSize {
        sigil_hal::LogicalSize::new(40.0, 16.0)
    }
}

fn key_down(key: Key, physical: PhysicalKey) -> ShellEvent {
    ShellEvent::Key(KeyEvent::new(KeyEventInit {
        key: key.into(),
        physical,
        modifiers: ModifiersState::empty(),
        repeat: false,
        phase: KeyPhase::Down,
        text: None,
    }))
}

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
        let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
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
