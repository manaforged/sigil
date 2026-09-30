#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used)]

#[path = "events_support/input.rs"]
mod input;
#[path = "widget_support/mod.rs"]
mod widget_support;

use input::pointer_move;
use sigil::events::EventRouter;
use sigil::events::PointerEventInit;
use sigil::events::ShellEvent;
use sigil::geometry::Point;
use sigil::kernel::Cx;
use sigil::prelude::*;
use sigil::style::Tone;
use sigil::style::scale::{ruby_light, slate_light};
use widget_support::{compute, rect_colors};

const VIEWPORT: Size = Size::fixed(640.0, 480.0);

#[test]
fn button_user_pressed_overlay_overrides_widget_pressed_bg() {
    sigil::reactive::Runtime::scope("ds2-button-pressed-overlay", || {
        let mut cx = Cx::new_root();
        let ruby_10 = ruby_light().step(10);
        let root = cx.add(button("Delete").pressed(|s| s.bg(ruby_10)));
        let layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
        let mut router = EventRouter::new();
        router.route(&mut cx, &layout, pointer_move(31.0, 14.0));
        router.route(
            &mut cx,
            &layout,
            ShellEvent::Pointer(PointerEvent::new(PointerEventInit {
                position: Point::new(31.0, 14.0),
                delta: Point::new(0.0, 0.0),
                kind: PointerKind::Mouse,
                button: Some(MouseButton::Left),
                phase: PointerPhase::Down,
                buttons: MouseButtons::LEFT,
                modifiers: ModifiersState::empty(),
            })),
        );

        let colors = rect_colors(&cx, root);
        assert!(
            colors.contains(&ruby_10),
            "user .pressed() bg must override widget pressed bg; \
             expected {ruby_10:?} in {colors:?}",
        );
    });
}

#[test]
fn button_user_rest_overlay_overrides_widget_rest_bg() {
    sigil::reactive::Runtime::scope("ds2-button-rest-overlay", || {
        let mut cx = Cx::new_root();
        let ruby_3 = ruby_light().step(3);
        let root = cx.add(button("Cancel").rest(|s| s.bg(ruby_3)));
        compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());

        let colors = rect_colors(&cx, root);
        assert!(
            colors.contains(&ruby_3),
            "user .rest() bg must override widget rest bg; \
             expected {ruby_3:?} in {colors:?}",
        );
    });
}

#[test]
fn button_destructive_sugar_matches_solid_error_tone() {
    sigil::reactive::Runtime::scope("ds2-5-button-destructive", || {
        let mut cx = Cx::new_root();
        let root = cx.add(button("Delete").destructive());
        compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());

        let error_9 = ruby_light().step(9);
        let colors = rect_colors(&cx, root);
        assert!(
            colors.contains(&error_9),
            ".destructive() must paint error-9 (the canonical destructive \
             CTA color); expected {error_9:?} in {colors:?}",
        );
    });
}

#[test]
fn button_solid_error_tone_paints_error_scale_step_9() {
    sigil::reactive::Runtime::scope("ds2-5-button-tone-error", || {
        let mut cx = Cx::new_root();
        let root = cx.add(button("Delete").solid().tone(Tone::Error));
        compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());

        let error_9 = ruby_light().step(9);
        let colors = rect_colors(&cx, root);
        assert!(
            colors.contains(&error_9),
            "solid + Tone::Error must paint error-9 via the tone-driven \
             SOLID_VARIANT; expected {error_9:?} in {colors:?}",
        );
    });
}

#[test]
fn button_without_overlay_keeps_widget_defaults() {
    sigil::reactive::Runtime::scope("ds2-button-no-overlay", || {
        let mut cx = Cx::new_root();
        let root = cx.add(button("OK"));
        compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
        let gray_4 = slate_light().step(4);
        let colors = rect_colors(&cx, root);
        assert!(
            colors.contains(&gray_4),
            "no overlay must leave widget defaults intact; \
             gray-4 {gray_4:?} not in {colors:?}",
        );
    });
}
