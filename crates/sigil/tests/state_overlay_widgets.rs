#![allow(clippy::unwrap_used)]

use crate::widget_support;

use sigil::events::Event;
use sigil::events::EventRouter;
use sigil::geometry::Point;
use sigil::kernel::{Cx, NodeId};
use sigil::layout::LayoutEngine;
use sigil::prelude::*;
use sigil::scene::{SceneCommand, emit_scene};
use sigil::style::Color;
use sigil::style::scale::ruby_light;
use widget_support::{compute, rect_colors};

const LEAVE_XY: (f32, f32) = (500.0, 400.0);
const VIEWPORT: Constraint = Constraint::fixed(640.0, 480.0);

fn assert_hover_overlay_paints(
    cx: &mut Cx,
    root: NodeId,
    layout: &LayoutEngine,
    hover_xy: (f32, f32),
    overlay: Color,
) {
    assert!(
        !rect_colors(cx, root).contains(&overlay),
        "rest scene must not contain the user hover colour {overlay:?}",
    );
    let mut router = EventRouter::new();
    router.route(
        cx,
        layout,
        Event::pointer_move(Point::new(hover_xy.0, hover_xy.1)),
    );
    assert!(
        rect_colors(cx, root).contains(&overlay),
        "pointer enter must paint the user .hover() overlay {overlay:?}; \
         scene colours: {:?}",
        rect_colors(cx, root),
    );
    router.route(
        cx,
        layout,
        Event::pointer_move(Point::new(LEAVE_XY.0, LEAVE_XY.1)),
    );
    assert!(
        !rect_colors(cx, root).contains(&overlay),
        "pointer leave must restore the base paint (drop {overlay:?})",
    );
}

#[test]
fn checkbox_user_hover_overlay_paints_row_root() {
    sigil::reactive::Owner::scope("t1-checkbox-hover-overlay", || {
        let checked = signal(false);
        let mut cx = Cx::new_root();
        let ruby_9 = ruby_light().step(9);
        let root = cx.add(checkbox(checked).hover(|s| s.background(ruby_9)));
        let layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
        assert_hover_overlay_paints(&mut cx, root, &layout, (10.0, 10.0), ruby_9);
    });
}

#[test]
fn dropdown_user_hover_overlay_paints_trigger() {
    sigil::reactive::Owner::scope("t1-dropdown-hover-overlay", || {
        let selected = signal(0_usize);
        let mut cx = Cx::new_root();
        let ruby_9 = ruby_light().step(9);
        let root = cx
            .add(dropdown(selected, vec!["a".into(), "b".into()]).hover(|s| s.background(ruby_9)));
        let layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
        assert_hover_overlay_paints(&mut cx, root, &layout, (100.0, 20.0), ruby_9);
    });
}

#[test]
fn slider_user_hover_overlay_paints_track_root() {
    sigil::reactive::Owner::scope("t1-slider-hover-overlay", || {
        let value = signal(50.0_f32);
        let mut cx = Cx::new_root();
        let ruby_9 = ruby_light().step(9);
        let root = cx.add(slider(value, 0.0..=100.0).hover(|s| s.background(ruby_9)));
        let layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
        assert_hover_overlay_paints(&mut cx, root, &layout, (10.0, 14.0), ruby_9);
    });
}

#[test]
fn tabs_user_hover_overlay_paints_hovered_tab() {
    sigil::reactive::Owner::scope("t1-tabs-hover-overlay", || {
        let active = signal(0_usize);
        let mut cx = Cx::new_root();
        let ruby_9 = ruby_light().step(9);
        let root = cx.add(
            tabs()
                .active(active)
                .labels(["A", "B"])
                .hover(|s| s.background(ruby_9)),
        );
        let layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
        assert_hover_overlay_paints(&mut cx, root, &layout, (120.0, 17.0), ruby_9);
    });
}

#[test]
fn text_field_user_hover_overlay_paints_frame() {
    sigil::reactive::Owner::scope("t1-text-field-hover-overlay", || {
        let value = signal(String::new());
        let mut cx = Cx::new_root();
        let ruby_9 = ruby_light().step(9);
        let root = cx.add(text_field(value).hover(|s| s.background(ruby_9)));
        let layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
        assert_hover_overlay_paints(&mut cx, root, &layout, (30.0, 15.0), ruby_9);
    });
}

#[test]
fn text_edit_user_hover_overlay_paints_frame() {
    sigil::reactive::Owner::scope("t1-text-edit-hover-overlay", || {
        let value = signal(String::new());
        let mut cx = Cx::new_root();
        let ruby_9 = ruby_light().step(9);
        let root = cx.add(
            text_field(value)
                .multiline()
                .hover(|s| s.background(ruby_9)),
        );
        let layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
        assert_hover_overlay_paints(&mut cx, root, &layout, (20.0, 20.0), ruby_9);
    });
}

#[test]
fn slider_thumb_hover_variant_intact_without_overlay() {
    sigil::reactive::Owner::scope("t1-slider-thumb-variant-intact", || {
        let value = signal(50.0_f32);
        let mut cx = Cx::new_root();
        let root = cx.add(slider(value, 0.0..=100.0));
        let layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
        let theme = Theme::default();
        let thumb_rim = |scene_cx: &Cx, scene_root: NodeId| {
            emit_scene(scene_cx, scene_root, None)
                .commands
                .iter()
                .find_map(|command| match command {
                    SceneCommand::Rect(rect) => rect.stroke.map(|stroke| stroke.color),
                    _ => None,
                })
        };
        assert_eq!(
            thumb_rim(&cx, root),
            Some(theme.color(Token::Accent)),
            "thumb rim must rest at accent"
        );
        let mut router = EventRouter::new();
        router.route(
            &mut cx,
            &layout,
            Event::pointer_move(Point::new(320.0, 12.0)),
        );
        assert_eq!(
            thumb_rim(&cx, root),
            Some(theme.color(Token::AccentHover)),
            "thumb rim must lift to accent-hover with no user overlay"
        );
    });
}
