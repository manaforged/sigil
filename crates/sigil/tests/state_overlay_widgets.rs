//! user state-overlay wiring on the remaining interactive widgets.
//!
//! Extends the `state_overlay_button.rs` input→pixel pattern to the six
//! builders whose `.hover()` / `.focus()` / `.pressed()` / `.disabled()`
//! closures were silent no-ops: `checkbox`, `dropdown`, `slider`, `tabs`,
//! `text_field`, `text_edit`. Each test drives a synthetic pointer enter and
//! asserts the emitted scene now contains the user overlay colour, then
//! drives a pointer leave and asserts the base value returns. A final
//! regression test proves a widget's own variant hover still resolves
//! when no user overlay is installed.

#![allow(missing_docs)]
#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used)]

use sigil::prelude::*;
use sigil_hal::{Point, PointerEventInit};
use sigil_renderer::{SceneCommand, emit_scene};
use sigil_style::Rgba8;
use sigil_style::scale::ruby_light;

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

/// Far outside every widget box used below — routes the pointer leave.
const LEAVE_XY: (f32, f32) = (500.0, 400.0);

/// Drive enter at `hover_xy`, assert `overlay` appears, then leave and
/// assert it disappears. The widget must already carry a `.hover()`
/// overlay for `overlay`.
fn assert_hover_overlay_paints(
    cx: &mut Cx,
    root: NodeId,
    layout: &LayoutEngine,
    hover_xy: (f32, f32),
    overlay: Rgba8,
) {
    assert!(
        !rect_colors(cx, root).contains(&overlay),
        "rest scene must not contain the user hover colour {overlay:?}",
    );
    let mut router = EventRouter::new();
    router.route(cx, layout, pointer_move_event(hover_xy.0, hover_xy.1));
    assert!(
        rect_colors(cx, root).contains(&overlay),
        "pointer enter must paint the user .hover() overlay {overlay:?}; \
         scene colours: {:?}",
        rect_colors(cx, root),
    );
    router.route(cx, layout, pointer_move_event(LEAVE_XY.0, LEAVE_XY.1));
    assert!(
        !rect_colors(cx, root).contains(&overlay),
        "pointer leave must restore the base paint (drop {overlay:?})",
    );
}

#[test]
fn checkbox_user_hover_overlay_paints_row_root() {
    sigil::reactive::Runtime::scope("t1-checkbox-hover-overlay", || {
        let checked = signal(false);
        let mut cx = Cx::new_root();
        let ruby_9 = ruby_light().step(9);
        let root = cx.add(checkbox(checked).hover(|s| s.bg(ruby_9)));
        let layout = compute(&mut cx, root);
        assert_hover_overlay_paints(&mut cx, root, &layout, (10.0, 10.0), ruby_9);
    });
}

#[test]
fn dropdown_user_hover_overlay_paints_trigger() {
    sigil::reactive::Runtime::scope("t1-dropdown-hover-overlay", || {
        let selected = signal(0_usize);
        let mut cx = Cx::new_root();
        let ruby_9 = ruby_light().step(9);
        let root = cx.add(dropdown(selected, vec!["a".into(), "b".into()]).hover(|s| s.bg(ruby_9)));
        let layout = compute(&mut cx, root);
        // (100, 20) is dead centre of the 200×40 trigger, clear of the
        // absolutely-positioned chevron swatch at the trailing edge.
        assert_hover_overlay_paints(&mut cx, root, &layout, (100.0, 20.0), ruby_9);
    });
}

#[test]
fn slider_user_hover_overlay_paints_track_root() {
    sigil::reactive::Runtime::scope("t1-slider-hover-overlay", || {
        let value = signal(50.0_f32);
        let mut cx = Cx::new_root();
        let ruby_9 = ruby_light().step(9);
        let root = cx.add(slider(value, 0.0..=100.0).hover(|s| s.bg(ruby_9)));
        let layout = compute(&mut cx, root);
        // (10, 14) sits on the track, well left of the centred thumb.
        assert_hover_overlay_paints(&mut cx, root, &layout, (10.0, 14.0), ruby_9);
    });
}

#[test]
fn tabs_user_hover_overlay_paints_hovered_tab() {
    sigil::reactive::Runtime::scope("t1-tabs-hover-overlay", || {
        let active = signal(0_usize);
        let mut cx = Cx::new_root();
        let ruby_9 = ruby_light().step(9);
        let root = cx.add(tabs(active, vec!["A".into(), "B".into()]).hover(|s| s.bg(ruby_9)));
        let layout = compute(&mut cx, root);
        // (120, 17) is inside tab B (tabs are 80×34 from the origin).
        assert_hover_overlay_paints(&mut cx, root, &layout, (120.0, 17.0), ruby_9);
    });
}

#[test]
fn text_field_user_hover_overlay_paints_frame() {
    sigil::reactive::Runtime::scope("t1-text-field-hover-overlay", || {
        let value = signal(String::new());
        let mut cx = Cx::new_root();
        let ruby_9 = ruby_light().step(9);
        let root = cx.add(text_field(value).hover(|s| s.bg(ruby_9)));
        let layout = compute(&mut cx, root);
        assert_hover_overlay_paints(&mut cx, root, &layout, (30.0, 15.0), ruby_9);
    });
}

#[test]
fn text_edit_user_hover_overlay_paints_frame() {
    sigil::reactive::Runtime::scope("t1-text-edit-hover-overlay", || {
        let value = signal(String::new());
        let mut cx = Cx::new_root();
        let ruby_9 = ruby_light().step(9);
        let root = cx.add(text_edit(value).hover(|s| s.bg(ruby_9)));
        let layout = compute(&mut cx, root);
        assert_hover_overlay_paints(&mut cx, root, &layout, (20.0, 20.0), ruby_9);
    });
}

#[test]
fn slider_thumb_hover_variant_intact_without_overlay() {
    // Regression: with no user overlay installed the widget's own
    // variant machinery must keep resolving — hovering the slider lifts
    // the thumb's rim from accent (THUMB_REST) to accent-hover (THUMB_HOVER),
    // layered by the same reactive channel the T1 wiring publishes the
    // overlay slot from. The thumb is the only stroked rect in the
    // slider scene, so the stroke color identifies it.
    sigil::reactive::Runtime::scope("t1-slider-thumb-variant-intact", || {
        let value = signal(50.0_f32);
        let mut cx = Cx::new_root();
        let root = cx.add(slider(value, 0.0..=100.0));
        let layout = compute(&mut cx, root);
        let theme = Theme::default();
        let thumb_rim = |scene_cx: &Cx, scene_root: NodeId| {
            emit_scene(scene_cx, scene_root)
                .commands
                .iter()
                .find_map(|command| match command {
                    SceneCommand::Rect(rect) => rect.stroke.map(|stroke| stroke.color),
                    _ => None,
                })
        };
        assert_eq!(
            thumb_rim(&cx, root),
            Some(theme.accent),
            "thumb rim must rest at accent"
        );
        let mut router = EventRouter::new();
        // (320, 12) is the horizontal centre of the value-50 thumb in
        // the fill-width root (24 px tall).
        router.route(&mut cx, &layout, pointer_move_event(320.0, 12.0));
        assert_eq!(
            thumb_rim(&cx, root),
            Some(theme.accent_hover),
            "thumb rim must lift to accent-hover with no user overlay"
        );
    });
}
