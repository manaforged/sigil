#![allow(missing_docs)]
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation
)]

use arboard as _;
use criterion as _;
use image as _;
use pollster as _;
use sigil_bidi as _;
use sigil_profile as _;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use wgpu as _;

use sigil_actions as _;
use sigil_animation as _;
use sigil_animation_runtime as _;
use sigil_app as _;
use sigil_events as _;
use sigil_kernel as _;
use sigil_reactive as _;
use sigil_tasks as _;
#[cfg(feature = "tui")]
use sigil_tui as _;
use sigil_widgets as _;

use sigil::prelude::*;
use sigil_hal::{KeyEventInit, Point, PointerEventInit};
use sigil_layout::{CaretGeometry, TextGeometry, set_text_geometry};
use sigil_renderer::{SceneCommand, emit_scene};
use sigil_style::TextStyle;

struct StubMeasure;

impl TextMeasure for StubMeasure {
    fn measure(&self, content: &TextContent, _space: sigil_hal::Size) -> sigil_hal::LogicalSize {
        let len = match content {
            TextContent::Static(s) => s.len(),
            TextContent::Reactive(sig) => sig.peek().len(),
        };
        sigil_hal::LogicalSize::new(len as f32 * 7.0, 16.0)
    }
}

/// Deterministic geometry: a uniform 10 px per byte (matches the editing
/// suite). Lets the scroll test assert pixel advance without real font metrics.
struct StubGeometry;

impl TextGeometry for StubGeometry {
    fn hit(&self, text: &str, _style: &TextStyle, x: f32) -> usize {
        ((x / 10.0).round().max(0.0) as usize).min(text.len())
    }

    fn caret(&self, _text: &str, _style: &TextStyle, byte: usize) -> CaretGeometry {
        CaretGeometry {
            x: byte as f32 * 10.0,
            top: 0.0,
            height: 16.0,
        }
    }
}

#[test]
fn button_label_accepts_a_signal() {
    sigil::reactive::Runtime::scope("sigil-widget-button-signal-label", || {
        let caption = signal(String::from("Save"));
        let mut cx = Cx::new_root();
        let root = cx.add(button(caption));
        let node = cx.get(root).expect("button");
        assert_eq!(node.a11y.role, Role::Button);
        let child_id = *node.children().first().expect("label text");
        let child = cx.get(child_id).expect("label text");
        match &child.kind {
            NodeType::Text { content, .. } => match content {
                TextContent::Reactive(value) => assert_eq!(value.get(), "Save"),
                TextContent::Static(value) => panic!("expected reactive label, got {value}"),
            },
            other => panic!("expected text child, got {other:?}"),
        }
        caption.set(String::from("Saving"));
        match &cx.get(child_id).expect("label text").kind {
            NodeType::Text {
                content: TextContent::Reactive(value),
                ..
            } => assert_eq!(value.get(), "Saving"),
            other => panic!("expected reactive label after set, got {other:?}"),
        }
    });
}

#[test]
fn button_is_focusable_and_activates_on_enter() {
    sigil::reactive::Runtime::scope("sigil-widget-button-enter", || {
        let hits = Rc::new(Cell::new(0_u32));
        let mut cx = Cx::new_root();
        let root = cx.add(button("Queue run").primary().activate({
            let hits = Rc::clone(&hits);
            move || hits.set(hits.get() + 1)
        }));
        let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
        layout
            .compute(&mut cx, root, Size::fixed(320.0, 120.0))
            .expect("layout computes");

        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(root));
        router.route(&mut cx, &layout, enter_down());

        assert_eq!(hits.get(), 1);
        let node = cx.get(root).unwrap();
        assert_eq!(node.a11y.role, Role::Button);
        assert!(node.a11y.focusable);
        assert!(node.get_extension::<FocusRing>().unwrap().visible());
    });
}

#[test]
fn disabled_button_does_not_focus_or_activate() {
    sigil::reactive::Runtime::scope("sigil-widget-button-disabled", || {
        let hits = Rc::new(Cell::new(0_u32));
        let mut cx = Cx::new_root();
        let root = cx.add(button("Disabled").enabled(false).activate({
            let hits = Rc::clone(&hits);
            move || hits.set(hits.get() + 1)
        }));
        let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
        layout
            .compute(&mut cx, root, Size::fixed(320.0, 120.0))
            .expect("layout computes");

        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(root));
        router.route(&mut cx, &layout, enter_down());

        assert_eq!(hits.get(), 0);
        let node = cx.get(root).unwrap();
        assert!(!node.a11y.focusable);
        assert!(!node.a11y.hittable);
    });
}

#[test]
fn button_pointer_activation_requires_press_inside() {
    sigil::reactive::Runtime::scope("sigil-widget-button-pointer", || {
        let hits = Rc::new(Cell::new(0_u32));
        let mut cx = Cx::new_root();
        let root = cx.add(button("Queue run").activate({
            let hits = Rc::clone(&hits);
            move || hits.set(hits.get() + 1)
        }));
        let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
        layout
            .compute(&mut cx, root, Size::fixed(320.0, 120.0))
            .expect("layout computes");

        let mut router = EventRouter::new();
        router.route(&mut cx, &layout, pointer_down(5.0, 5.0));
        router.route(&mut cx, &layout, pointer_up(5.0, 5.0));
        router.route(&mut cx, &layout, pointer_down(5.0, 5.0));
        router.route(&mut cx, &layout, pointer_move(500.0, 500.0));
        router.route(&mut cx, &layout, pointer_up(500.0, 500.0));

        assert_eq!(hits.get(), 1);
    });
}

#[test]
fn label_accepts_reactive_text() {
    sigil::reactive::Runtime::scope("sigil-widget-label-reactive", || {
        let value = signal(String::from("Ready"));
        let mut cx = Cx::new_root();
        let root = cx.add(label(value));
        let node = cx.get(root).unwrap();
        let is_reactive_text = matches!(&node.kind, NodeType::Text { content, .. } if content.as_static_str().is_none());
        assert!(is_reactive_text);
    });
}

#[test]
fn text_field_typing_updates_signal() {
    sigil::reactive::Runtime::scope("sigil-widget-text-field-typing", || {
        let value = signal(String::new());
        let mut cx = Cx::new_root();
        let root = cx.add(text_field(value).placeholder("Notes"));
        let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
        layout
            .compute(&mut cx, root, Size::fixed(320.0, 120.0))
            .expect("layout computes");

        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(root));
        router.route(&mut cx, &layout, text_down("a"));
        router.route(&mut cx, &layout, text_down("é"));

        assert_eq!(value.get(), "aé");
        let node = cx.get(root).unwrap();
        assert_eq!(node.a11y.role, Role::TextInput);
        assert_eq!(node.a11y.text_value.expect("text value").get(), "aé");
        assert_eq!(node.a11y.placeholder.as_deref(), Some("Notes"));
    });
}

#[test]
fn text_field_typing_emits_the_updated_glyph_run() {
    sigil::reactive::Runtime::scope("sigil-widget-text-field-typing-paint", || {
        let value = signal(String::new());
        let mut cx = Cx::new_root();
        let root = cx.add(text_field(value).placeholder("Notes"));
        let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
        layout
            .compute(&mut cx, root, Size::fixed(320.0, 120.0))
            .expect("layout computes");

        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(root));
        router.route(&mut cx, &layout, text_down("a"));
        layout
            .compute(&mut cx, root, Size::fixed(320.0, 120.0))
            .expect("layout recomputes");

        assert_eq!(value.get(), "a", "typing must update editing state");
        assert!(
            scene_has_text(&cx, root, "a"),
            "typing must replace the placeholder glyph run with the edited value"
        );
    });
}

#[test]
fn text_field_backspace_removes_last_character() {
    sigil::reactive::Runtime::scope("sigil-widget-text-field-backspace", || {
        let value = signal(String::from("ab"));
        let mut cx = Cx::new_root();
        let root = cx.add(text_field(value));
        let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
        layout
            .compute(&mut cx, root, Size::fixed(320.0, 120.0))
            .expect("layout computes");

        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(root));
        router.route(&mut cx, &layout, backspace_down());

        assert_eq!(value.get(), "a");
    });
}

#[test]
fn text_field_disabled_ignores_typing() {
    sigil::reactive::Runtime::scope("sigil-widget-text-field-disabled", || {
        let value = signal(String::new());
        let disabled = signal(true);
        let mut cx = Cx::new_root();
        let root = cx.add(text_field(value).disabled(disabled));
        let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
        layout
            .compute(&mut cx, root, Size::fixed(320.0, 120.0))
            .expect("layout computes");

        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(root));
        router.route(&mut cx, &layout, text_down("a"));

        assert_eq!(value.get(), "");
        let node = cx.get(root).unwrap();
        assert!(!node.a11y.focusable);
        assert!(!node.a11y.hittable);
    });
}

#[test]
fn text_field_enter_fires_submit_with_current_value() {
    sigil::reactive::Runtime::scope("sigil-widget-text-field-submit", || {
        let value = signal(String::from("ready"));
        let submitted = Rc::new(RefCell::new(String::new()));
        let mut cx = Cx::new_root();
        let root = cx.add(text_field(value).submit({
            let submitted = Rc::clone(&submitted);
            move |text: &str| {
                submitted.replace(text.to_owned());
            }
        }));
        let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
        layout
            .compute(&mut cx, root, Size::fixed(320.0, 120.0))
            .expect("layout computes");

        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(root));
        router.route(&mut cx, &layout, enter_down());

        assert_eq!(submitted.borrow().as_str(), "ready");
        assert_eq!(value.get(), "ready");
    });
}

#[test]
fn text_field_shift_arrow_selects_and_cut_clears_value() {
    sigil::reactive::Runtime::scope("sigil-widget-text-field-selection", || {
        let value = signal(String::from("hello"));
        let mut cx = Cx::new_root();
        let root = cx.add(text_field(value));
        let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
        layout
            .compute(&mut cx, root, Size::fixed(320.0, 120.0))
            .expect("layout computes");

        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(root));
        // Home → shift+right×5 to select "hello"
        router.route(&mut cx, &layout, named_key(Key::Home, PhysicalKey::Home));
        for _ in 0..5 {
            router.route(
                &mut cx,
                &layout,
                shift_arrow(Key::ArrowRight, PhysicalKey::ArrowRight),
            );
        }
        // Ctrl+X cuts the selection — field should now be empty
        router.route(&mut cx, &layout, ctrl_key("x", PhysicalKey::KeyX));
        assert_eq!(value.get(), "");
    });
}

#[test]
fn text_field_ctrl_a_selects_all_and_delete_clears() {
    sigil::reactive::Runtime::scope("sigil-widget-text-field-select-all", || {
        let value = signal(String::from("hello world"));
        let mut cx = Cx::new_root();
        let root = cx.add(text_field(value));
        let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
        layout
            .compute(&mut cx, root, Size::fixed(320.0, 120.0))
            .expect("layout computes");

        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(root));
        router.route(&mut cx, &layout, ctrl_key("a", PhysicalKey::KeyA));
        router.route(&mut cx, &layout, backspace_down());
        assert_eq!(value.get(), "");
    });
}

#[test]
fn text_field_placeholder_renders_when_empty_and_unfocused() {
    sigil::reactive::Runtime::scope("sigil-widget-text-field-placeholder", || {
        let value = signal(String::new());
        let mut cx = Cx::new_root();
        let root = cx.add(text_field(value).placeholder("Add note"));
        let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
        layout
            .compute(&mut cx, root, Size::fixed(320.0, 120.0))
            .expect("layout computes");

        let scene = emit_scene(&cx, root);
        let placeholder = scene
            .commands
            .iter()
            .find_map(|command| match command {
                SceneCommand::GlyphRun(run) if run.run.text == "Add note" => Some(&run.run),
                _ => None,
            })
            .expect("placeholder glyph run");

        assert_eq!(placeholder.color, Theme::default().text_muted());
    });
}

#[test]
fn text_field_placeholder_remains_visible_when_focused() {
    sigil::reactive::Runtime::scope("sigil-widget-text-field-placeholder-focus", || {
        let value = signal(String::new());
        let mut cx = Cx::new_root();
        let root = cx.add(text_field(value).placeholder("Add note"));
        let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
        layout
            .compute(&mut cx, root, Size::fixed(320.0, 120.0))
            .expect("layout computes");

        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(root));
        layout
            .compute(&mut cx, root, Size::fixed(320.0, 120.0))
            .expect("layout recomputes");

        assert!(scene_has_text(&cx, root, "Add note"));
    });
}

#[test]
fn text_field_focus_ring_visible_when_focused() {
    sigil::reactive::Runtime::scope("sigil-widget-text-field-focus-ring", || {
        let value = signal(String::new());
        let mut cx = Cx::new_root();
        let root = cx.add(text_field(value).placeholder("Notes"));
        let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
        layout
            .compute(&mut cx, root, Size::fixed(320.0, 120.0))
            .expect("layout computes");

        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(root));

        let node = cx.get(root).unwrap();
        assert!(node.get_extension::<FocusRing>().unwrap().visible());
    });
}

#[test]
fn checkbox_click_flips_signal() {
    sigil::reactive::Runtime::scope("sigil-widget-checkbox-click", || {
        let checked = signal(false);
        let changes = Rc::new(Cell::new(0_u32));
        let mut cx = Cx::new_root();
        let root = cx.add(checkbox(checked).label("Restock alerts").change({
            let changes = Rc::clone(&changes);
            move |next| {
                assert!(next);
                changes.set(changes.get() + 1);
            }
        }));
        let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
        layout
            .compute(&mut cx, root, Size::fixed(320.0, 120.0))
            .expect("layout computes");

        let mut router = EventRouter::new();
        router.route(&mut cx, &layout, pointer_down(5.0, 5.0));
        router.route(&mut cx, &layout, pointer_up(5.0, 5.0));

        assert!(checked.get());
        assert_eq!(changes.get(), 1);
        let node = cx.get(root).unwrap();
        assert_eq!(node.a11y.role, Role::CheckBox);
        assert!(node.a11y.checked.unwrap().get());
    });
}

#[test]
fn checkbox_keyboard_activation() {
    sigil::reactive::Runtime::scope("sigil-widget-checkbox-keyboard", || {
        let checked = signal(false);
        let mut cx = Cx::new_root();
        let root = cx.add(checkbox(checked).label("Monitor"));
        let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
        layout
            .compute(&mut cx, root, Size::fixed(320.0, 120.0))
            .expect("layout computes");

        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(root));
        router.route(&mut cx, &layout, space_down());
        assert!(checked.get());

        router.route(&mut cx, &layout, enter_down());
        assert!(!checked.get());
    });
}

#[test]
fn checkbox_disabled_ignores_click() {
    sigil::reactive::Runtime::scope("sigil-widget-checkbox-disabled", || {
        let checked = signal(false);
        let disabled = signal(true);
        let changes = Rc::new(Cell::new(0_u32));
        let mut cx = Cx::new_root();
        let root = cx.add(
            checkbox(checked)
                .label("Disabled")
                .disabled(disabled)
                .change({
                    let changes = Rc::clone(&changes);
                    move |_| changes.set(changes.get() + 1)
                }),
        );
        let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
        layout
            .compute(&mut cx, root, Size::fixed(320.0, 120.0))
            .expect("layout computes");

        let mut router = EventRouter::new();
        router.route(&mut cx, &layout, pointer_down(5.0, 5.0));
        router.route(&mut cx, &layout, pointer_up(5.0, 5.0));

        assert!(!checked.get());
        assert_eq!(changes.get(), 0);
        let node = cx.get(root).unwrap();
        assert!(!node.a11y.focusable);
        assert!(!node.a11y.hittable);
    });
}

#[test]
fn checkbox_scene_emits_checked_and_unchecked_variants() {
    sigil::reactive::Runtime::scope("sigil-widget-checkbox-scene", || {
        let checked = signal(false);
        let mut cx = Cx::new_root();
        let root = cx.add(checkbox(checked).label("Visual").tone(Tone::Primary));
        let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
        layout
            .compute(&mut cx, root, Size::fixed(320.0, 120.0))
            .expect("layout computes");

        let theme = Theme::default();
        let unchecked_colors = scene_rect_colors(&cx, root);
        // Unchecked fill is the `surface-inset` well — gray step 1.
        assert!(unchecked_colors.contains(&theme.gray.step(1)));

        checked.set(true);
        let checked_colors = scene_rect_colors(&cx, root);
        assert!(checked_colors.contains(&theme.accent));
        // The check mark is a white stroked vector rasterized to an
        // `Image` command — visibility is its opacity, not a rect
        // colour. Checked means the mark paints at full opacity.
        assert!(
            emit_scene(&cx, root)
                .commands
                .iter()
                .any(|command| matches!(
                    command,
                    SceneCommand::Image(image) if image.opacity == 10_000
                )),
            "checked scene must paint the white vector check mark"
        );
    });
}

#[test]
fn toggle_click_flips_signal() {
    sigil::reactive::Runtime::scope("sigil-widget-toggle-click", || {
        let on = signal(false);
        let changes = Rc::new(Cell::new(0_u32));
        let mut cx = Cx::new_root();
        let root = cx.add(toggle(on).label("Watch").change({
            let changes = Rc::clone(&changes);
            move |next| {
                assert!(next);
                changes.set(changes.get() + 1);
            }
        }));
        let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
        layout
            .compute(&mut cx, root, Size::fixed(320.0, 120.0))
            .expect("layout computes");

        let mut router = EventRouter::new();
        router.route(&mut cx, &layout, pointer_down(5.0, 5.0));
        router.route(&mut cx, &layout, pointer_up(5.0, 5.0));

        assert!(on.get());
        assert_eq!(changes.get(), 1);
        let node = cx.get(root).unwrap();
        assert_eq!(node.a11y.role, Role::Switch);
        assert!(node.a11y.checked.unwrap().get());
    });
}

#[test]
fn toggle_keyboard_activation() {
    sigil::reactive::Runtime::scope("sigil-widget-toggle-keyboard", || {
        let on = signal(false);
        let mut cx = Cx::new_root();
        let root = cx.add(toggle(on).label("Monitor"));
        let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
        layout
            .compute(&mut cx, root, Size::fixed(320.0, 120.0))
            .expect("layout computes");

        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(root));
        router.route(&mut cx, &layout, space_down());
        assert!(on.get());

        router.route(&mut cx, &layout, enter_down());
        assert!(!on.get());
    });
}

#[test]
fn toggle_scene_emits_on_and_off_variants() {
    sigil::reactive::Runtime::scope("sigil-widget-toggle-scene", || {
        let on = signal(false);
        let mut cx = Cx::new_root();
        let root = cx.add(toggle(on).label("Visual"));
        let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
        layout
            .compute(&mut cx, root, Size::fixed(320.0, 120.0))
            .expect("layout computes");

        let theme = Theme::default();
        let off_colors = scene_rect_colors(&cx, root);
        let off_thumb_x = first_rect_x_for_color(&cx, root, theme.on_accent);
        assert!(off_colors.contains(&theme.surface_hover));

        on.set(true);
        layout
            .compute(&mut cx, root, Size::fixed(320.0, 120.0))
            .expect("layout recomputes");
        let on_colors = scene_rect_colors(&cx, root);
        let on_thumb_x = first_rect_x_for_color(&cx, root, theme.on_accent);
        // ON track paints the accent scale at step 8.
        assert!(on_colors.contains(&theme.accent_scale.step(8)));
        assert!(on_thumb_x > off_thumb_x);
    });
}

#[test]
fn toggle_and_checkbox_share_styled_interaction_path() {
    let interaction_src = include_str!("../../widgets/src/interaction.rs");
    let shared_src = include_str!("../../widgets/src/shared.rs");
    let checkbox_src = include_str!("../../widgets/src/checkbox.rs");
    let toggle_src = include_str!("../../widgets/src/toggle.rs");

    assert!(interaction_src.contains("pub fn style_state"));
    assert!(interaction_src.contains("pub fn release_pointer"));
    assert!(shared_src.contains("interaction.release_pointer"));
    assert!(shared_src.contains("interaction.activate_key"));
    assert!(shared_src.contains("binary_style_state"));
    assert!(checkbox_src.contains("binary_style_state"));
    assert!(toggle_src.contains("binary_style_state"));
}

#[test]
fn slider_click_jumps_to_position() {
    sigil::reactive::Runtime::scope("sigil-widget-slider-click", || {
        let value = signal(0.0_f32);
        let mut cx = Cx::new_root();
        let root = cx.add(slider(value, 0.0..=100.0).length(SLIDER_TRACK));
        let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
        layout
            .compute(&mut cx, root, Size::fixed(320.0, 120.0))
            .expect("layout computes");

        let mut router = EventRouter::new();
        router.route(&mut cx, &layout, pointer_down(90.0, 10.0));

        assert_approx(value.get(), 50.0);
        let node = cx.get(root).unwrap();
        assert_eq!(node.a11y.role, Role::Slider);
        assert_approx(node.a11y.value_now.unwrap().get(), 50.0);
        assert_eq!(node.a11y.value_min, Some(0.0));
        assert_eq!(node.a11y.value_max, Some(100.0));
    });
}

#[test]
fn slider_drag_updates_value_continuously() {
    sigil::reactive::Runtime::scope("sigil-widget-slider-drag", || {
        let value = signal(0.0_f32);
        let mut cx = Cx::new_root();
        let root = cx.add(slider(value, 0.0..=100.0).length(SLIDER_TRACK));
        let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
        layout
            .compute(&mut cx, root, Size::fixed(320.0, 120.0))
            .expect("layout computes");

        let mut router = EventRouter::new();
        router.route(&mut cx, &layout, pointer_down(0.0, 10.0));
        router.route(&mut cx, &layout, pointer_move(135.0, 10.0));

        assert_approx(value.get(), 75.0);
    });
}

#[test]
fn slider_drag_with_step_quantizes_value() {
    sigil::reactive::Runtime::scope("sigil-widget-slider-step", || {
        let value = signal(0.0_f32);
        let mut cx = Cx::new_root();
        let root = cx.add(slider(value, 0.0..=100.0).step(10.0).length(SLIDER_TRACK));
        let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
        layout
            .compute(&mut cx, root, Size::fixed(320.0, 120.0))
            .expect("layout computes");

        let mut router = EventRouter::new();
        router.route(&mut cx, &layout, pointer_down(47.0, 10.0));
        router.route(&mut cx, &layout, pointer_move(64.0, 10.0));

        assert_approx(value.get(), 40.0);
    });
}

#[test]
fn slider_keyboard_stepping() {
    sigil::reactive::Runtime::scope("sigil-widget-slider-keyboard", || {
        let value = signal(50.0_f32);
        let mut cx = Cx::new_root();
        let root = cx.add(slider(value, 0.0..=100.0).step(5.0).length(SLIDER_TRACK));
        let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
        layout
            .compute(&mut cx, root, Size::fixed(320.0, 120.0))
            .expect("layout computes");

        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(root));
        router.route(
            &mut cx,
            &layout,
            arrow_down(Key::ArrowRight, PhysicalKey::ArrowRight),
        );
        assert_approx(value.get(), 55.0);

        router.route(
            &mut cx,
            &layout,
            arrow_down(Key::ArrowLeft, PhysicalKey::ArrowLeft),
        );
        assert_approx(value.get(), 50.0);
    });
}

#[test]
fn slider_disabled_ignores_input() {
    sigil::reactive::Runtime::scope("sigil-widget-slider-disabled", || {
        let value = signal(25.0_f32);
        let disabled = signal(true);
        let mut cx = Cx::new_root();
        let root = cx.add(
            slider(value, 0.0..=100.0)
                .disabled(disabled)
                .step(5.0)
                .length(SLIDER_TRACK),
        );
        let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
        layout
            .compute(&mut cx, root, Size::fixed(320.0, 120.0))
            .expect("layout computes");

        let mut router = EventRouter::new();
        router.route(&mut cx, &layout, pointer_down(180.0, 10.0));
        router.set_focus(&mut cx, Some(root));
        router.route(
            &mut cx,
            &layout,
            arrow_down(Key::ArrowRight, PhysicalKey::ArrowRight),
        );

        assert_approx(value.get(), 25.0);
        let node = cx.get(root).unwrap();
        assert!(!node.a11y.focusable);
        assert!(!node.a11y.hittable);
    });
}

#[test]
fn slider_capture_releases_on_pointer_up() {
    sigil::reactive::Runtime::scope("sigil-widget-slider-capture-release", || {
        let value = signal(0.0_f32);
        let changes = Rc::new(Cell::new(0_u32));
        let mut cx = Cx::new_root();
        let root = cx.add(slider(value, 0.0..=100.0).length(SLIDER_TRACK).change({
            let changes = Rc::clone(&changes);
            move |_| changes.set(changes.get() + 1)
        }));
        let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
        layout
            .compute(&mut cx, root, Size::fixed(320.0, 120.0))
            .expect("layout computes");

        let mut router = EventRouter::new();
        router.route(&mut cx, &layout, pointer_down(0.0, 10.0));
        router.route(&mut cx, &layout, pointer_move(180.0, 10.0));
        router.route(&mut cx, &layout, pointer_up(180.0, 10.0));
        assert_approx(value.get(), 100.0);
        assert_eq!(changes.get(), 1);

        router.route(&mut cx, &layout, pointer_move(0.0, 10.0));
        assert_approx(value.get(), 100.0);
        assert_eq!(changes.get(), 1);
    });
}

#[test]
fn text_field_undo_restores_previous_value() {
    sigil::reactive::Runtime::scope("sigil-widget-text-field-undo", || {
        let value = signal(String::new());
        let builder = text_field(value);
        let mut cx = Cx::new_root();
        let root = cx.add(builder);
        let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
        layout
            .compute(&mut cx, root, Size::fixed(320.0, 120.0))
            .expect("layout computes");

        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(root));
        router.route(&mut cx, &layout, type_char("a", PhysicalKey::KeyA));
        router.route(&mut cx, &layout, type_char("b", PhysicalKey::KeyB));
        assert_eq!(value.get(), "ab");

        router.route(&mut cx, &layout, ctrl_key("z", PhysicalKey::KeyZ));
        assert_eq!(value.get(), "a", "undo should restore value before 'b'");

        router.route(&mut cx, &layout, ctrl_key("z", PhysicalKey::KeyZ));
        assert_eq!(value.get(), "", "second undo should restore empty");
    });
}

#[test]
fn text_field_redo_reapplies_undone_change() {
    sigil::reactive::Runtime::scope("sigil-widget-text-field-redo", || {
        let value = signal(String::new());
        let mut cx = Cx::new_root();
        let root = cx.add(text_field(value));
        let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
        layout
            .compute(&mut cx, root, Size::fixed(320.0, 120.0))
            .expect("layout computes");

        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(root));
        router.route(&mut cx, &layout, type_char("x", PhysicalKey::KeyX));
        assert_eq!(value.get(), "x");

        router.route(&mut cx, &layout, ctrl_key("z", PhysicalKey::KeyZ));
        assert_eq!(value.get(), "");

        router.route(&mut cx, &layout, ctrl_shift_key("z", PhysicalKey::KeyZ));
        assert_eq!(value.get(), "x", "redo should reapply 'x'");
    });
}

#[test]
fn text_field_caret_visible_resets_on_keystroke() {
    sigil::reactive::Runtime::scope("sigil-widget-text-field-caret-blink", || {
        let value = signal(String::new());
        let mut builder = text_field(value);
        let caret_visible = builder.caret_visible();
        let mut cx = Cx::new_root();
        let root = cx.add(builder);
        let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
        layout
            .compute(&mut cx, root, Size::fixed(320.0, 120.0))
            .expect("layout computes");

        assert!(caret_visible.get(), "starts visible");

        caret_visible.set(false);
        assert!(!caret_visible.get());

        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(root));
        router.route(&mut cx, &layout, type_char("k", PhysicalKey::KeyK));
        assert!(
            caret_visible.get(),
            "keystroke resets caret_visible to true"
        );
    });
}

#[test]
fn text_field_scroll_offset_advances_past_visible_window() {
    sigil::reactive::Runtime::scope("sigil-widget-text-field-scroll", || {
        // Pixel scroll is driven by real caret geometry; install the stub so
        // the caret x is known (10 px/byte) instead of staying at 0.
        set_text_geometry(Rc::new(StubGeometry));
        let value = signal(String::new());
        let mut builder = text_field(value);
        let scroll_offset = builder.scroll_offset();
        let mut cx = Cx::new_root();
        let root = cx.add(builder);
        let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
        layout
            .compute(&mut cx, root, Size::fixed(320.0, 120.0))
            .expect("layout computes");

        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(root));

        for ch in "abcdefghijklmnopqrstuvwxyz".chars() {
            let key_str = ch.to_string();
            router.route(
                &mut cx,
                &layout,
                type_char_owned(key_str, PhysicalKey::KeyA),
            );
        }
        // 26 chars × 10 px = 260 px caret, past the 156 px inner column, so the
        // track must have scrolled right (260 − 156 = 104 px) to keep it in view.
        assert!(
            scroll_offset.get() > 0.0,
            "scroll_offset should advance after long input, got {}",
            scroll_offset.get()
        );
    });
}

fn ctrl_shift_key(ch: &str, physical: PhysicalKey) -> ShellEvent {
    ShellEvent::Key(KeyEvent::new(KeyEventInit {
        key: LogicalKey::character(ch),
        physical,
        modifiers: ModifiersState::CONTROL | ModifiersState::SHIFT,
        repeat: false,
        phase: KeyPhase::Down,
        text: None,
    }))
}

fn type_char(ch: &str, physical: PhysicalKey) -> ShellEvent {
    ShellEvent::Key(KeyEvent::new(KeyEventInit {
        key: LogicalKey::character(ch),
        physical,
        modifiers: ModifiersState::empty(),
        repeat: false,
        phase: KeyPhase::Down,
        text: Some(ch.to_string().into()),
    }))
}

fn type_char_owned(ch: String, physical: PhysicalKey) -> ShellEvent {
    ShellEvent::Key(KeyEvent::new(KeyEventInit {
        key: LogicalKey::character(ch.as_str()),
        physical,
        modifiers: ModifiersState::empty(),
        repeat: false,
        phase: KeyPhase::Down,
        text: Some(ch.into()),
    }))
}

fn scene_rect_colors(cx: &Cx, root: NodeId) -> Vec<sigil_style::Rgba8> {
    emit_scene(cx, root)
        .commands
        .iter()
        .filter_map(|command| match command {
            SceneCommand::Rect(rect) => Some(rect.color),
            _ => None,
        })
        .collect()
}

fn first_rect_x_for_color(cx: &Cx, root: NodeId, color: sigil_style::Rgba8) -> f32 {
    emit_scene(cx, root)
        .commands
        .iter()
        .find_map(|command| match command {
            SceneCommand::Rect(rect) if rect.color == color => Some(rect.rect.origin.x),
            _ => None,
        })
        .expect("rect with requested color exists")
}

fn scene_has_text(cx: &Cx, root: NodeId, expected: &str) -> bool {
    emit_scene(cx, root).find_text(expected).is_some()
}

const fn pointer_down(x: f32, y: f32) -> ShellEvent {
    pointer_event(
        x,
        y,
        PointerPhase::Down,
        Some(MouseButton::Left),
        MouseButtons::LEFT,
    )
}

const fn pointer_move(x: f32, y: f32) -> ShellEvent {
    pointer_event(x, y, PointerPhase::Move, None, MouseButtons::LEFT)
}

const fn pointer_up(x: f32, y: f32) -> ShellEvent {
    pointer_event(
        x,
        y,
        PointerPhase::Up,
        Some(MouseButton::Left),
        MouseButtons::empty(),
    )
}

const fn pointer_event(
    x: f32,
    y: f32,
    phase: PointerPhase,
    button: Option<MouseButton>,
    buttons: MouseButtons,
) -> ShellEvent {
    ShellEvent::Pointer(PointerEvent::new(PointerEventInit {
        position: Point::new(x, y),
        delta: Point::new(0.0, 0.0),
        kind: PointerKind::Mouse,
        button,
        buttons,
        modifiers: ModifiersState::empty(),
        phase,
    }))
}

fn text_down(text: &str) -> ShellEvent {
    ShellEvent::Key(KeyEvent::new(KeyEventInit {
        key: LogicalKey::character(text),
        physical: PhysicalKey::KeyA,
        modifiers: ModifiersState::empty(),
        repeat: false,
        phase: KeyPhase::Down,
        text: Some(text.into()),
    }))
}

fn backspace_down() -> ShellEvent {
    ShellEvent::Key(KeyEvent::new(KeyEventInit {
        key: Key::Backspace.into(),
        physical: PhysicalKey::Backspace,
        modifiers: ModifiersState::empty(),
        repeat: false,
        phase: KeyPhase::Down,
        text: None,
    }))
}

fn enter_down() -> ShellEvent {
    ShellEvent::Key(KeyEvent::new(KeyEventInit {
        key: Key::Enter.into(),
        physical: PhysicalKey::Enter,
        modifiers: ModifiersState::empty(),
        repeat: false,
        phase: KeyPhase::Down,
        text: None,
    }))
}

fn space_down() -> ShellEvent {
    ShellEvent::Key(KeyEvent::new(KeyEventInit {
        key: LogicalKey::character(" "),
        physical: PhysicalKey::Space,
        modifiers: ModifiersState::empty(),
        repeat: false,
        phase: KeyPhase::Down,
        text: None,
    }))
}

fn arrow_down(key: Key, physical: PhysicalKey) -> ShellEvent {
    ShellEvent::Key(KeyEvent::new(KeyEventInit {
        key: key.into(),
        physical,
        modifiers: ModifiersState::empty(),
        repeat: false,
        phase: KeyPhase::Down,
        text: None,
    }))
}

/// Historic default slider track width. These tests verify pointer->value
/// mapping and their click positions are calibrated against it, so they pin
/// it explicitly now that a bare slider fills its parent instead.
const SLIDER_TRACK: f32 = 180.0;

fn assert_approx(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 0.001,
        "expected {expected}, got {actual}"
    );
}

fn named_key(key: Key, physical: PhysicalKey) -> ShellEvent {
    ShellEvent::Key(KeyEvent::new(KeyEventInit {
        key: key.into(),
        physical,
        modifiers: ModifiersState::empty(),
        repeat: false,
        phase: KeyPhase::Down,
        text: None,
    }))
}

fn shift_arrow(key: Key, physical: PhysicalKey) -> ShellEvent {
    ShellEvent::Key(KeyEvent::new(KeyEventInit {
        key: key.into(),
        physical,
        modifiers: ModifiersState::SHIFT,
        repeat: false,
        phase: KeyPhase::Down,
        text: None,
    }))
}

fn ctrl_key(ch: &str, physical: PhysicalKey) -> ShellEvent {
    ShellEvent::Key(KeyEvent::new(KeyEventInit {
        key: LogicalKey::character(ch),
        physical,
        modifiers: ModifiersState::CONTROL,
        repeat: false,
        phase: KeyPhase::Down,
        text: None,
    }))
}
