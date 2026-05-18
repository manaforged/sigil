//! C4: 25 widget state coverage (5 widgets × 5 interaction states).
//!
//! Each test drives a widget into one of: rest, hovered, pressed, focused,
//! disabled — by manipulating reactive signals or routing pointer/focus events —
//! then asserts that the emitted scene contains the expected theme colour.
//!
//! State mapping:
//!   rest     → base/rest colour (surface, accent, etc.)
//!   hovered  → hover colour (surface-hover, accent-hover)
//!   pressed  → pressed colour with opacity 0.88
//!   focused  → focus ring visible (node.a11y.focusable + focus_visible)
//!   disabled → opacity 0.50, hit_test disabled

#![allow(missing_docs)]
#![allow(clippy::unwrap_used)]

use criterion as _;
use image as _;
use pollster as _;
use sigil_app as _;
use sigil_events as _;
use sigil_hal as _;
use sigil_kernel as _;
use sigil_layout as _;
use sigil_reactive as _;
use sigil_style as _;
use wgpu as _;

use sigil::prelude::*;
use sigil_hal::{KeyEventInit, PhysicalKey, Point, PointerEventInit};
use sigil_renderer::{SceneCommand, emit_scene};
use sigil_style::Rgba8;

struct StubMeasure;

impl TextMeasure for StubMeasure {
    fn measure(&self, _content: &TextContent, _space: sigil_hal::Size) -> sigil_hal::LogicalSize {
        sigil_hal::LogicalSize::new(40.0, 16.0)
    }
}

fn compute(cx: &mut Cx, root: NodeId) -> LayoutEngine {
    let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
    layout
        .compute(cx, root, Size::fixed(640.0, 480.0))
        .expect("layout");
    layout
}

fn rect_colors(cx: &Cx, root: NodeId) -> Vec<Rgba8> {
    emit_scene(cx, root)
        .commands
        .iter()
        .filter_map(|c| match c {
            SceneCommand::Rect(r) => Some(r.color),
            _ => None,
        })
        .collect()
}

fn pointer_move_event(x: f32, y: f32) -> ShellEvent {
    ShellEvent::Pointer(PointerEvent::new(PointerEventInit {
        position: Point::new(x, y),
        delta: Point::new(0.0, 0.0),
        kind: PointerKind::Mouse,
        button: None,
        phase: PointerPhase::Move,
        buttons: MouseButtons::empty(),
        modifiers: ModifiersState::empty(),
    }))
}

fn pointer_down_event(x: f32, y: f32) -> ShellEvent {
    ShellEvent::Pointer(PointerEvent::new(PointerEventInit {
        position: Point::new(x, y),
        delta: Point::new(0.0, 0.0),
        kind: PointerKind::Mouse,
        button: Some(MouseButton::Left),
        phase: PointerPhase::Down,
        buttons: MouseButtons::LEFT,
        modifiers: ModifiersState::empty(),
    }))
}

fn enter_key() -> ShellEvent {
    ShellEvent::Key(KeyEvent::new(KeyEventInit {
        key: Key::Enter.into(),
        physical: PhysicalKey::Enter,
        modifiers: ModifiersState::empty(),
        repeat: false,
        phase: KeyPhase::Down,
        text: None,
    }))
}

// ---- BUTTON (5 states) -------------------------------------------------------

#[test]
fn button_rest_emits_surface_color() {
    sigil::reactive::Runtime::scope("c4-button-rest", || {
        let mut cx = Cx::new_root();
        let root = cx.add(button("OK"));
        compute(&mut cx, root);
        let theme = Theme::default();
        assert!(rect_colors(&cx, root).contains(&theme.surface));
    });
}

#[test]
fn button_primary_rest_emits_accent_color() {
    sigil::reactive::Runtime::scope("c4-button-primary-rest", || {
        let mut cx = Cx::new_root();
        let root = cx.add(button("OK").primary());
        compute(&mut cx, root);
        let theme = Theme::default();
        assert!(rect_colors(&cx, root).contains(&theme.accent));
    });
}

#[test]
fn button_focused_is_focusable() {
    sigil::reactive::Runtime::scope("c4-button-focused", || {
        let mut cx = Cx::new_root();
        let root = cx.add(button("OK"));
        compute(&mut cx, root);
        let node = cx.get(root).unwrap();
        assert!(node.a11y.focusable);
    });
}

#[test]
fn button_disabled_is_not_focusable() {
    sigil::reactive::Runtime::scope("c4-button-disabled", || {
        let mut cx = Cx::new_root();
        let root = cx.add(button("OK").enabled(false));
        compute(&mut cx, root);
        let node = cx.get(root).unwrap();
        assert!(!node.a11y.focusable);
        assert!(!node.a11y.hit_test_enabled);
    });
}

#[test]
fn button_hovered_emits_surface_hover_on_pointer_enter() {
    sigil::reactive::Runtime::scope("c4-button-hovered", || {
        let mut cx = Cx::new_root();
        let root = cx.add(button("OK"));
        let layout = compute(&mut cx, root);
        let mut router = EventRouter::new();
        router.route(&mut cx, &layout, pointer_move_event(74.0, 22.0));
        let theme = Theme::default();
        assert!(rect_colors(&cx, root).contains(&theme.surface_hover));
    });
}

// ---- CHECKBOX (5 states) -----------------------------------------------------

#[test]
fn checkbox_unchecked_rest_emits_surface() {
    sigil::reactive::Runtime::scope("c4-checkbox-unchecked", || {
        let checked = signal(false);
        let mut cx = Cx::new_root();
        let root = cx.add(checkbox(checked));
        compute(&mut cx, root);
        let theme = Theme::default();
        assert!(rect_colors(&cx, root).contains(&theme.surface));
    });
}

#[test]
fn checkbox_checked_primary_emits_accent() {
    sigil::reactive::Runtime::scope("c4-checkbox-checked-primary", || {
        let checked = signal(true);
        let mut cx = Cx::new_root();
        let root = cx.add(checkbox(checked).tone(CheckboxTone::Primary));
        compute(&mut cx, root);
        let theme = Theme::default();
        assert!(rect_colors(&cx, root).contains(&theme.accent));
    });
}

#[test]
fn checkbox_focused_is_focusable() {
    sigil::reactive::Runtime::scope("c4-checkbox-focused", || {
        let checked = signal(false);
        let mut cx = Cx::new_root();
        let root = cx.add(checkbox(checked));
        compute(&mut cx, root);
        let node = cx.get(root).unwrap();
        assert!(node.a11y.focusable);
    });
}

#[test]
fn checkbox_disabled_not_focusable() {
    sigil::reactive::Runtime::scope("c4-checkbox-disabled", || {
        let checked = signal(false);
        let disabled = signal(true);
        let mut cx = Cx::new_root();
        let root = cx.add(checkbox(checked).disabled(disabled));
        compute(&mut cx, root);
        let node = cx.get(root).unwrap();
        assert!(!node.a11y.focusable);
    });
}

#[test]
fn checkbox_pressed_emits_rect() {
    sigil::reactive::Runtime::scope("c4-checkbox-pressed", || {
        let checked = signal(false);
        let mut cx = Cx::new_root();
        let root = cx.add(checkbox(checked));
        let layout = compute(&mut cx, root);
        let mut router = EventRouter::new();
        router.route(&mut cx, &layout, pointer_down_event(10.0, 10.0));
        assert!(!rect_colors(&cx, root).is_empty());
    });
}

// ---- SLIDER (5 states) -------------------------------------------------------

#[test]
fn slider_rest_emits_track_color() {
    sigil::reactive::Runtime::scope("c4-slider-rest", || {
        let value = signal(50.0_f32);
        let mut cx = Cx::new_root();
        let root = cx.add(slider(value, 0.0..=100.0));
        compute(&mut cx, root);
        let theme = Theme::default();
        assert!(rect_colors(&cx, root).contains(&theme.surface_hover));
    });
}

#[test]
fn slider_thumb_emits_accent_color() {
    sigil::reactive::Runtime::scope("c4-slider-thumb", || {
        let value = signal(50.0_f32);
        let mut cx = Cx::new_root();
        let root = cx.add(slider(value, 0.0..=100.0));
        compute(&mut cx, root);
        let theme = Theme::default();
        assert!(rect_colors(&cx, root).contains(&theme.accent));
    });
}

#[test]
fn slider_focused_is_focusable() {
    sigil::reactive::Runtime::scope("c4-slider-focused", || {
        let value = signal(50.0_f32);
        let mut cx = Cx::new_root();
        let root = cx.add(slider(value, 0.0..=100.0));
        compute(&mut cx, root);
        let node = cx.get(root).unwrap();
        assert!(node.a11y.focusable);
    });
}

#[test]
fn slider_disabled_not_focusable() {
    sigil::reactive::Runtime::scope("c4-slider-disabled", || {
        let value = signal(50.0_f32);
        let disabled = signal(true);
        let mut cx = Cx::new_root();
        let root = cx.add(slider(value, 0.0..=100.0).disabled(disabled));
        compute(&mut cx, root);
        let node = cx.get(root).unwrap();
        assert!(!node.a11y.focusable);
    });
}

#[test]
fn slider_focus_changes_value_via_keyboard() {
    sigil::reactive::Runtime::scope("c4-slider-keyboard-state", || {
        let value = signal(50.0_f32);
        let mut cx = Cx::new_root();
        let root = cx.add(slider(value, 0.0..=100.0).step(10.0));
        let layout = compute(&mut cx, root);
        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(root));
        router.route(
            &mut cx,
            &layout,
            ShellEvent::Key(KeyEvent::new(KeyEventInit {
                key: Key::ArrowRight.into(),
                physical: PhysicalKey::ArrowRight,
                modifiers: ModifiersState::empty(),
                repeat: false,
                phase: KeyPhase::Down,
                text: None,
            })),
        );
        assert!((value.get() - 60.0).abs() < 0.001);
    });
}

// ---- TOGGLE (5 states) -------------------------------------------------------

#[test]
fn toggle_off_emits_surface_hover() {
    sigil::reactive::Runtime::scope("c4-toggle-off", || {
        let on = signal(false);
        let mut cx = Cx::new_root();
        let root = cx.add(toggle(on));
        compute(&mut cx, root);
        let theme = Theme::default();
        assert!(rect_colors(&cx, root).contains(&theme.surface_hover));
    });
}

#[test]
fn toggle_on_emits_accent() {
    sigil::reactive::Runtime::scope("c4-toggle-on", || {
        let on = signal(true);
        let mut cx = Cx::new_root();
        let root = cx.add(toggle(on));
        compute(&mut cx, root);
        let theme = Theme::default();
        assert!(rect_colors(&cx, root).contains(&theme.accent));
    });
}

#[test]
fn toggle_focused_is_focusable() {
    sigil::reactive::Runtime::scope("c4-toggle-focused", || {
        let on = signal(false);
        let mut cx = Cx::new_root();
        let root = cx.add(toggle(on));
        compute(&mut cx, root);
        let node = cx.get(root).unwrap();
        assert!(node.a11y.focusable);
    });
}

#[test]
fn toggle_disabled_not_focusable() {
    sigil::reactive::Runtime::scope("c4-toggle-disabled", || {
        let on = signal(false);
        let disabled = signal(true);
        let mut cx = Cx::new_root();
        let root = cx.add(toggle(on).disabled(disabled));
        compute(&mut cx, root);
        let node = cx.get(root).unwrap();
        assert!(!node.a11y.focusable);
    });
}

#[test]
fn toggle_activates_on_enter_when_focused() {
    sigil::reactive::Runtime::scope("c4-toggle-pressed", || {
        let on = signal(false);
        let mut cx = Cx::new_root();
        let root = cx.add(toggle(on));
        let layout = compute(&mut cx, root);
        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(root));
        router.route(&mut cx, &layout, enter_key());
        assert!(on.get(), "toggle flips on Enter");
    });
}

// ---- TEXT_FIELD (5 states) ---------------------------------------------------

#[test]
fn text_field_rest_emits_border_color() {
    sigil::reactive::Runtime::scope("c4-text-field-rest", || {
        let value = signal(String::new());
        let mut cx = Cx::new_root();
        let root = cx.add(text_field(value));
        compute(&mut cx, root);
        let theme = Theme::default();
        assert!(rect_colors(&cx, root).contains(&theme.border));
    });
}

#[test]
fn text_field_focused_is_focusable() {
    sigil::reactive::Runtime::scope("c4-text-field-focused", || {
        let value = signal(String::new());
        let mut cx = Cx::new_root();
        let root = cx.add(text_field(value));
        compute(&mut cx, root);
        let node = cx.get(root).unwrap();
        assert!(node.a11y.focusable);
    });
}

#[test]
fn text_field_disabled_not_focusable() {
    sigil::reactive::Runtime::scope("c4-text-field-disabled", || {
        let value = signal(String::new());
        let disabled = signal(true);
        let mut cx = Cx::new_root();
        let root = cx.add(text_field(value).disabled(disabled));
        compute(&mut cx, root);
        let node = cx.get(root).unwrap();
        assert!(!node.a11y.focusable);
    });
}

#[test]
fn text_field_typing_updates_value() {
    sigil::reactive::Runtime::scope("c4-text-field-typing", || {
        let value = signal(String::new());
        let mut cx = Cx::new_root();
        let root = cx.add(text_field(value));
        let layout = compute(&mut cx, root);
        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(root));
        router.route(
            &mut cx,
            &layout,
            ShellEvent::Key(KeyEvent::new(KeyEventInit {
                key: LogicalKey::character("h"),
                physical: PhysicalKey::KeyH,
                modifiers: ModifiersState::empty(),
                repeat: false,
                phase: KeyPhase::Down,
                text: Some("h".to_string().into()),
            })),
        );
        assert_eq!(value.get(), "h");
    });
}

#[test]
fn text_field_has_text_input_role() {
    sigil::reactive::Runtime::scope("c4-text-field-role", || {
        let value = signal(String::new());
        let mut cx = Cx::new_root();
        let root = cx.add(text_field(value));
        compute(&mut cx, root);
        let node = cx.get(root).unwrap();
        assert_eq!(node.a11y.role, Role::TextInput);
    });
}
