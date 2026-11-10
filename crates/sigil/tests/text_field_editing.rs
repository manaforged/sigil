#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::expect_used)]
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]

#[path = "events_support/input.rs"]
mod input;
#[path = "widget_support/mod.rs"]
mod widget_support;

use sigil::events::Event;
use sigil::events::EventRouter;
use sigil::events::PhysicalKey;
use sigil::geometry::Point;
use sigil::kernel::Cx;
use sigil::layout::set_text_geometry;
use sigil::prelude::*;
use widget_support::compute;

const VIEWPORT: Size = Size::fixed(320.0, 120.0);

#[test]
fn text_field_undo_reverts_typing() {
    sigil::reactive::Owner::scope("text-field-editing-undo", || {
        let value = signal(String::new());
        let mut cx = Cx::new_root();
        let root = cx.add(text_field(value));
        let layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());

        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(root));
        router.route(
            &mut cx,
            &layout,
            Event::typed("a").physical(PhysicalKey::KeyA),
        );
        router.route(
            &mut cx,
            &layout,
            Event::typed("b").physical(PhysicalKey::KeyB),
        );
        assert_eq!(value.get(), "ab");

        router.route(&mut cx, &layout, ctrl_key("z", PhysicalKey::KeyZ));

        assert_eq!(value.get(), "a");
    });
}

#[test]
fn text_field_redo_reapplies_undone_edit() {
    sigil::reactive::Owner::scope("text-field-editing-redo", || {
        let value = signal(String::new());
        let mut cx = Cx::new_root();
        let root = cx.add(text_field(value));
        let layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());

        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(root));
        router.route(
            &mut cx,
            &layout,
            Event::typed("x").physical(PhysicalKey::KeyX),
        );
        router.route(&mut cx, &layout, ctrl_key("z", PhysicalKey::KeyZ));
        assert_eq!(value.get(), "");

        router.route(&mut cx, &layout, ctrl_key("y", PhysicalKey::KeyY));

        assert_eq!(value.get(), "x");
    });
}

#[test]
fn text_field_select_all_then_type_replaces_buffer() {
    sigil::reactive::Owner::scope("text-field-editing-select-all-type", || {
        let value = signal(String::from("hello world"));
        let mut cx = Cx::new_root();
        let root = cx.add(text_field(value));
        let layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());

        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(root));
        router.route(&mut cx, &layout, ctrl_key("a", PhysicalKey::KeyA));
        router.route(
            &mut cx,
            &layout,
            Event::typed("q").physical(PhysicalKey::KeyQ),
        );

        assert_eq!(value.get(), "q");
    });
}

#[test]
fn text_field_pointer_drag_selects_then_type_replaces() {
    sigil::reactive::Owner::scope("text-field-pointer-drag", || {
        set_text_geometry(std::rc::Rc::new(sigil::layout::FakeMeasure::default()));
        let value = signal(String::from("hello world"));
        let mut cx = Cx::new_root();
        let root = cx.add(text_field(value));
        let layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());

        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(root));
        router.route(
            &mut cx,
            &layout,
            Event::pointer_down(Point::new(14.0, 18.0)),
        );
        router.route(
            &mut cx,
            &layout,
            Event::pointer_drag(Point::new(64.0, 18.0)),
        );
        router.route(&mut cx, &layout, Event::pointer_up(Point::new(64.0, 18.0)));
        router.route(
            &mut cx,
            &layout,
            Event::typed("q").physical(PhysicalKey::KeyQ),
        );

        assert_eq!(
            value.get(),
            "q world",
            "drag-selected 'hello' should be replaced by the typed 'q'"
        );
    });
}

#[test]
fn text_field_external_write_resyncs_edit_buffer() {
    sigil::reactive::Owner::scope("text-field-editing-external-write", || {
        let value = signal(String::new());
        let mut cx = Cx::new_root();
        let root = cx.add(text_field(value));
        let layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());

        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(root));
        router.route(
            &mut cx,
            &layout,
            Event::typed("a").physical(PhysicalKey::KeyA),
        );
        router.route(
            &mut cx,
            &layout,
            Event::typed("b").physical(PhysicalKey::KeyB),
        );
        assert_eq!(value.get(), "ab");

        value.set(String::from("cleared"));
        router.route(
            &mut cx,
            &layout,
            Event::key(Key::Backspace, KeyPhase::Down).physical(PhysicalKey::Backspace),
        );

        assert_eq!(
            value.get(),
            "ceared",
            "backspace must edit the externally-written buffer, not the stale 'ab'"
        );
    });
}

fn ctrl_key(ch: &str, physical: PhysicalKey) -> Event {
    Event::key(LogicalKey::character(ch), KeyPhase::Down)
        .physical(physical)
        .modifiers(ModifiersState::CONTROL)
}
