#![allow(clippy::unwrap_used)]

use crate::widget_support;

use sigil::events::Event;
use sigil::events::EventRouter;
use sigil::events::PhysicalKey;
use sigil::geometry::Point;
use sigil::kernel::Cx;
use sigil::prelude::*;
use sigil::scene::{SceneCommand, StrokeAlign, emit_scene};
use sigil::style::scale::ScaleRole;
use widget_support::{assert_focus_follows_disabled, compute, rect_colors};

const VIEWPORT: Size = Size::fixed(640.0, 480.0);

const fn pointer_down_event(x: f32, y: f32) -> Event {
    Event::pointer(PointerPhase::Down, Point::new(x, y))
        .button(Some(MouseButton::Left))
        .buttons(MouseButtons::LEFT)
}

fn enter_key() -> Event {
    Event::key(Key::Enter, KeyPhase::Down).physical(PhysicalKey::Enter)
}

#[test]
fn button_rest_emits_surface_color() {
    sigil::reactive::Owner::scope("c4-button-rest", || {
        let mut cx = Cx::new_root();
        let root = cx.add(button("OK"));
        compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
        let expected = sigil::style::scale::slate_light().step(4);
        assert!(rect_colors(&cx, root).contains(&expected));
    });
}

#[test]
fn button_primary_rest_emits_accent_color() {
    sigil::reactive::Owner::scope("c4-button-primary-rest", || {
        let mut cx = Cx::new_root();
        let root = cx.add(button("OK").primary());
        compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
        let theme = Theme::default();
        assert!(rect_colors(&cx, root).contains(&theme.accent));
    });
}

#[test]
fn button_focused_is_focusable() {
    sigil::reactive::Owner::scope("c4-button-focused", || {
        let mut cx = Cx::new_root();
        let root = cx.add(button("OK"));
        compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
        let node = cx.get(root).unwrap();
        assert!(node.a11y.focusable);
    });
}

#[test]
fn button_disabled_is_not_focusable() {
    sigil::reactive::Owner::scope("c4-button-disabled", || {
        let mut cx = Cx::new_root();
        let root = cx.add(button("OK").disabled(true));
        let layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
        let mut router = EventRouter::new();
        assert!(!router.is_focus_target(&cx, &layout, root));
        assert!(!router.is_click_target(&cx, &layout, root));
    });
}

#[test]
fn button_hovered_emits_soft_hover_on_pointer_enter() {
    sigil::reactive::Owner::scope("c4-button-hovered", || {
        let mut cx = Cx::new_root();
        let root = cx.add(button("OK"));
        let layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
        let mut router = EventRouter::new();
        router.route(
            &mut cx,
            &layout,
            Event::pointer_move(Point::new(24.0, 16.0)),
        );
        let theme = Theme::default();
        assert!(rect_colors(&cx, root).contains(&theme.gray.step(5)));
    });
}

#[test]
fn checkbox_unchecked_rest_emits_surface_inset() {
    sigil::reactive::Owner::scope("c4-checkbox-unchecked", || {
        let checked = signal(false);
        let mut cx = Cx::new_root();
        let root = cx.add(checkbox(checked));
        compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
        let theme = Theme::default();
        assert!(rect_colors(&cx, root).contains(&theme.gray.step(1)));
    });
}

#[test]
fn checkbox_checked_primary_emits_accent() {
    sigil::reactive::Owner::scope("c4-checkbox-checked-primary", || {
        let checked = signal(true);
        let mut cx = Cx::new_root();
        let root = cx.add(checkbox(checked).tone(Tone::Primary));
        compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
        let theme = Theme::default();
        assert!(rect_colors(&cx, root).contains(&theme.accent));
    });
}

#[test]
fn checkbox_focused_is_focusable() {
    sigil::reactive::Owner::scope("c4-checkbox-focused", || {
        let checked = signal(false);
        let mut cx = Cx::new_root();
        let root = cx.add(checkbox(checked));
        compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
        let node = cx.get(root).unwrap();
        assert!(node.a11y.focusable);
    });
}

#[test]
fn checkbox_disabled_not_focusable() {
    sigil::reactive::Owner::scope("c4-checkbox-disabled", || {
        let checked = signal(false);
        let disabled = signal(true);
        let mut cx = Cx::new_root();
        let root = cx.add(checkbox(checked).disabled(disabled));
        let layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
        assert_focus_follows_disabled(&mut cx, &layout, root, disabled);
    });
}

#[test]
fn checkbox_pressed_emits_rect() {
    sigil::reactive::Owner::scope("c4-checkbox-pressed", || {
        let checked = signal(false);
        let mut cx = Cx::new_root();
        let root = cx.add(checkbox(checked));
        let layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
        let mut router = EventRouter::new();
        router.route(&mut cx, &layout, pointer_down_event(10.0, 10.0));
        assert!(!rect_colors(&cx, root).is_empty());
    });
}

#[test]
fn slider_rest_emits_track_color() {
    sigil::reactive::Owner::scope("c4-slider-rest", || {
        let value = signal(50.0_f32);
        let mut cx = Cx::new_root();
        let root = cx.add(slider(value, 0.0..=100.0));
        compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
        let theme = Theme::default();
        assert!(rect_colors(&cx, root).contains(&theme.gray.step(4)));
    });
}

#[test]
fn slider_thumb_emits_accent_color() {
    sigil::reactive::Owner::scope("c4-slider-thumb", || {
        let value = signal(50.0_f32);
        let mut cx = Cx::new_root();
        let root = cx.add(slider(value, 0.0..=100.0));
        compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
        let theme = Theme::default();
        assert!(rect_colors(&cx, root).contains(&theme.accent));
    });
}

#[test]
fn slider_focused_is_focusable() {
    sigil::reactive::Owner::scope("c4-slider-focused", || {
        let value = signal(50.0_f32);
        let mut cx = Cx::new_root();
        let root = cx.add(slider(value, 0.0..=100.0));
        compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
        let node = cx.get(root).unwrap();
        assert!(node.a11y.focusable);
    });
}

#[test]
fn slider_disabled_not_focusable() {
    sigil::reactive::Owner::scope("c4-slider-disabled", || {
        let value = signal(50.0_f32);
        let disabled = signal(true);
        let mut cx = Cx::new_root();
        let root = cx.add(slider(value, 0.0..=100.0).disabled(disabled));
        let layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
        assert_focus_follows_disabled(&mut cx, &layout, root, disabled);
    });
}

#[test]
fn slider_focus_changes_value_via_keyboard() {
    sigil::reactive::Owner::scope("c4-slider-keyboard-state", || {
        let value = signal(50.0_f32);
        let mut cx = Cx::new_root();
        let root = cx.add(slider(value, 0.0..=100.0).step(10.0));
        let layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(root));
        router.route(
            &mut cx,
            &layout,
            Event::key(Key::ArrowRight, KeyPhase::Down).physical(PhysicalKey::ArrowRight),
        );
        assert!((value.get() - 60.0).abs() < 0.001);
    });
}

#[test]
fn toggle_off_emits_surface_hover() {
    sigil::reactive::Owner::scope("c4-toggle-off", || {
        let on = signal(false);
        let mut cx = Cx::new_root();
        let root = cx.add(toggle(on));
        compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
        let theme = Theme::default();
        assert!(rect_colors(&cx, root).contains(&theme.color(sigil::style::Token::Gray6)));
    });
}

#[test]
fn toggle_on_emits_accent() {
    sigil::reactive::Owner::scope("c4-toggle-on", || {
        let on = signal(true);
        let mut cx = Cx::new_root();
        let root = cx.add(toggle(on));
        compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
        let theme = Theme::default();
        assert!(rect_colors(&cx, root).contains(&theme.accent));
    });
}

#[test]
fn toggle_focused_is_focusable() {
    sigil::reactive::Owner::scope("c4-toggle-focused", || {
        let on = signal(false);
        let mut cx = Cx::new_root();
        let root = cx.add(toggle(on));
        compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
        let node = cx.get(root).unwrap();
        assert!(node.a11y.focusable);
    });
}

#[test]
fn toggle_disabled_not_focusable() {
    sigil::reactive::Owner::scope("c4-toggle-disabled", || {
        let on = signal(false);
        let disabled = signal(true);
        let mut cx = Cx::new_root();
        let root = cx.add(toggle(on).disabled(disabled));
        let layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
        assert_focus_follows_disabled(&mut cx, &layout, root, disabled);
    });
}

#[test]
fn toggle_activates_on_enter_when_focused() {
    sigil::reactive::Owner::scope("c4-toggle-pressed", || {
        let on = signal(false);
        let mut cx = Cx::new_root();
        let root = cx.add(toggle(on));
        let layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(root));
        router.route(&mut cx, &layout, enter_key());
        assert!(on.get(), "toggle flips on Enter");
    });
}

#[test]
fn text_field_rest_emits_border_color() {
    sigil::reactive::Owner::scope("c4-text-field-rest", || {
        let value = signal(String::new());
        let mut cx = Cx::new_root();
        let root = cx.add(text_field(value));
        compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
        let theme = Theme::default();
        let border = theme.scale(ScaleRole::Gray).step(7);
        let scene = emit_scene(&cx, root, None);
        let rects = scene
            .commands
            .iter()
            .filter_map(|command| match command {
                SceneCommand::Rect(rect) => Some(rect),
                _ => None,
            })
            .collect::<Vec<_>>();

        assert_eq!(rects.len(), 1, "field must paint one rounded surface");
        let rect = rects.first().expect("field surface");
        let stroke = rect.stroke.expect("field border stroke");
        assert_eq!(rect.color, theme.gray.step(1));
        assert_eq!(stroke.color, border);
        assert!((stroke.width - 1.0).abs() < f32::EPSILON);
        assert_eq!(stroke.align, StrokeAlign::Inside);
    });
}

#[test]
fn text_field_focused_is_focusable() {
    sigil::reactive::Owner::scope("c4-text-field-focused", || {
        let value = signal(String::new());
        let mut cx = Cx::new_root();
        let root = cx.add(text_field(value));
        compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
        let node = cx.get(root).unwrap();
        assert!(node.a11y.focusable);
    });
}

#[test]
fn text_field_disabled_not_focusable() {
    sigil::reactive::Owner::scope("c4-text-field-disabled", || {
        let value = signal(String::new());
        let disabled = signal(true);
        let mut cx = Cx::new_root();
        let root = cx.add(text_field(value).disabled(disabled));
        let layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
        assert_focus_follows_disabled(&mut cx, &layout, root, disabled);
    });
}

#[test]
fn text_field_typing_updates_value() {
    sigil::reactive::Owner::scope("c4-text-field-typing", || {
        let value = signal(String::new());
        let mut cx = Cx::new_root();
        let root = cx.add(text_field(value));
        let layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(root));
        router.route(
            &mut cx,
            &layout,
            Event::key(LogicalKey::character("h"), KeyPhase::Down)
                .physical(PhysicalKey::KeyH)
                .text(Some("h".to_string())),
        );
        assert_eq!(value.get(), "h");
    });
}

#[test]
fn text_field_has_text_input_role() {
    sigil::reactive::Owner::scope("c4-text-field-role", || {
        let value = signal(String::new());
        let mut cx = Cx::new_root();
        let root = cx.add(text_field(value));
        compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
        let node = cx.get(root).unwrap();
        assert_eq!(node.a11y.role, Role::TextInput);
    });
}
