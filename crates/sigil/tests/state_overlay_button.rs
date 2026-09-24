#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used)]

use sigil::hal::{Point, PointerEventInit};
use sigil::prelude::*;
use sigil::renderer::{SceneCommand, emit_scene};
use sigil::style::Rgba8;
use sigil::style::Tone;
use sigil::style::scale::{ruby_light, slate_light};

struct StubMeasure;
impl TextMeasure for StubMeasure {
    fn measure(&self, _content: &TextContent, _space: sigil::hal::Size) -> sigil::hal::LogicalSize {
        sigil::hal::LogicalSize::new(40.0, 16.0)
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

#[test]
fn button_user_pressed_overlay_overrides_widget_pressed_bg() {
    sigil::reactive::Runtime::scope("ds2-button-pressed-overlay", || {
        let mut cx = Cx::new_root();
        let ruby_10 = ruby_light().step(10);
        let root = cx.add(button("Delete").pressed(|s| s.bg(ruby_10)));
        let layout = compute(&mut cx, root);
        let mut router = EventRouter::new();
        router.route(&mut cx, &layout, pointer_move_event(31.0, 14.0));
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
        let gray_4 = slate_light().step(4);
        let colors = rect_colors(&cx, root);
        assert!(
            colors.contains(&gray_4),
            "no overlay must leave widget defaults intact; \
             gray-4 {gray_4:?} not in {colors:?}",
        );
    });
}
