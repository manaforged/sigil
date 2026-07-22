//! DS2 Phase 2 — user state-overlay merging on widgets.
//!
//! Driving `.hover(|s| s.bg(error_9))` on a button should override the
//! widget's default hover bg with `error_9` at runtime. The button's
//! `into_node_builder` drains the user's state overlays from the
//! `Style` accumulator and merges them into the widget's
//! `StyleVariant`; the renderer's last-atom-wins resolution then
//! lets user atoms beat widget atoms per property.

#![allow(missing_docs)]
#![allow(unused_crate_dependencies)]
#![allow(clippy::unwrap_used)]

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
use sigil_tasks as _;
use wgpu as _;

use sigil::prelude::*;
use sigil_hal::{Point, PointerEventInit};
use sigil_renderer::{SceneCommand, emit_scene};
use sigil_style::Rgba8;
use sigil_style::Tone;
use sigil_style::scale::{ruby_light, slate_light};

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

const fn pointer_move_event(x: f32, y: f32) -> ShellEvent {
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

// NOTE: live hover/pressed render-comparison is tracked as DS2 Phase 2.5
// — the transition animator currently intercepts bg color resolution
// and interpolates from its baked source/target. After this commit's
// `variant.merge_overlay(user)` change the animator IS aware of the
// merged state slot, but the per-frame interpolation may still hold a
// non-user color when the test reads colors immediately after the
// pointer event. The structural merge below proves the StateStyle path;
// the live integration acceptance test goes in `state_overlay_button_live`.

// NOTE: the `disabled` name collision is resolved (5/5 Wave 1, D-U2):
// the behavioral form keeps `.disabled(Signal<bool>)` (the common call,
// and the one rustc's inherent-method preference already picked), and
// the style-closure form is `.disabled_style(|s| ...)` on StateVariants.
// This codifies in the API what method resolution was already doing.

#[test]
fn button_user_pressed_overlay_overrides_widget_pressed_bg() {
    sigil::reactive::Runtime::scope("ds2-button-pressed-overlay", || {
        let mut cx = Cx::new_root();
        let ruby_10 = ruby_light().step(10);
        let root = cx.add(button("Delete").pressed(|s| s.bg(ruby_10)));
        let layout = compute(&mut cx, root);
        // Drive into hover, then press.
        let mut router = EventRouter::new();
        router.route(&mut cx, &layout, pointer_move_event(74.0, 22.0));
        router.route(
            &mut cx,
            &layout,
            ShellEvent::Pointer(PointerEvent::new(PointerEventInit {
                position: Point::new(74.0, 22.0),
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
        // No interaction event — pure rest state.
        let root = cx.add(button("Cancel").rest(|s| s.bg(ruby_3)));
        compute(&mut cx, root);

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
        // `.destructive()` is sugar for `.solid().tone(Tone::Error)`
        // — both paths must paint the same error-9 background.
        let root = cx.add(button("Delete").destructive());
        compute(&mut cx, root);

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
        // The DS2.5 destructive-button canonical pattern — solid
        // variant + Tone::Error resolves through the theme's error
        // scale at step 9 (Radix solid).
        let root = cx.add(button("Delete").solid().tone(Tone::Error));
        compute(&mut cx, root);

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
        compute(&mut cx, root);
        // Default button is soft + neutral — Radix step 3 of the gray
        // scale (interactive subtle bg).
        let gray_3 = slate_light().step(3);
        let colors = rect_colors(&cx, root);
        assert!(
            colors.contains(&gray_3),
            "no overlay must leave widget defaults intact; \
             gray-3 {gray_3:?} not in {colors:?}",
        );
    });
}
