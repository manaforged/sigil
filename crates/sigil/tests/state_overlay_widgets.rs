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
use sigil::prelude::*;
use sigil::scene::{SceneCommand, emit_scene};
use sigil::style::Rgba8;
use sigil::style::scale::ruby_light;
use widget_support::{compute, rect_colors};

const LEAVE_XY: (f32, f32) = (500.0, 400.0);
const VIEWPORT: Size = Size::fixed(640.0, 480.0);

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
    router.route(cx, layout, pointer_move(hover_xy.0, hover_xy.1));
    assert!(
        rect_colors(cx, root).contains(&overlay),
        "pointer enter must paint the user .hover() overlay {overlay:?}; \
         scene colours: {:?}",
        rect_colors(cx, root),
    );
    router.route(cx, layout, pointer_move(LEAVE_XY.0, LEAVE_XY.1));
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
        let layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
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
        let layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
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
        let layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
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
        let layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
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
        let layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
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
        let layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
        assert_hover_overlay_paints(&mut cx, root, &layout, (20.0, 20.0), ruby_9);
    });
}

#[test]
fn slider_thumb_hover_variant_intact_without_overlay() {
    sigil::reactive::Runtime::scope("t1-slider-thumb-variant-intact", || {
        let value = signal(50.0_f32);
        let mut cx = Cx::new_root();
        let root = cx.add(slider(value, 0.0..=100.0));
        let layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
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
        router.route(&mut cx, &layout, pointer_move(320.0, 12.0));
        assert_eq!(
            thumb_rim(&cx, root),
            Some(theme.accent_hover),
            "thumb rim must lift to accent-hover with no user overlay"
        );
    });
}
