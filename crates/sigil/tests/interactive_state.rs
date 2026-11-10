#![allow(clippy::unwrap_used)]

use crate::widget_support;

use sigil::events::Event;
use sigil::events::EventRouter;
use sigil::geometry::Point;
use sigil::kernel::Cx;
use sigil::prelude::*;
use sigil::reactive::Owner;
use sigil::style::{Color, Theme};
use widget_support::{compute, rect_colors};

const HOVER_BG: Color = Color {
    r: 10,
    g: 200,
    b: 30,
    a: 255,
};
const PRESSED_BG: Color = Color {
    r: 200,
    g: 20,
    b: 200,
    a: 255,
};
const REST_BG: Color = Color {
    r: 240,
    g: 240,
    b: 240,
    a: 255,
};
const VIEWPORT: Size = Size::fixed(640.0, 480.0);

const fn pointer_down_event(x: f32, y: f32) -> Event {
    Event::pointer(PointerPhase::Down, Point::new(x, y))
        .button(Some(sigil::events::MouseButton::Left))
}

fn rest_pane() -> NodeBuilder {
    container()
        .layout(LayoutStyle::block().fixed_size(100.0, 100.0))
        .background(REST_BG)
}

#[test]
fn interactive_hover_overlay_paints_on_pointer_enter() {
    Owner::scope("interactive-hover-paints", || {
        let mut cx = Cx::new_root();
        let root = cx.add(interactive(rest_pane()).hover(|s| s.background(HOVER_BG)));
        let layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
        let mut router = EventRouter::new();

        assert!(
            !rect_colors(&cx, root).contains(&HOVER_BG),
            "rest state must not paint the hover slice"
        );

        router.route(
            &mut cx,
            &layout,
            Event::pointer_move(Point::new(50.0, 50.0)),
        );
        assert!(
            rect_colors(&cx, root).contains(&HOVER_BG),
            "pointer enter must paint the .hover() slice"
        );

        router.route(
            &mut cx,
            &layout,
            Event::pointer_move(Point::new(300.0, 300.0)),
        );
        assert!(
            !rect_colors(&cx, root).contains(&HOVER_BG),
            "pointer leave must return to rest"
        );
    });
}

#[test]
fn interactive_class_hover_prefix_paints_on_pointer_enter() {
    Owner::scope("interactive-class-hover", || {
        let mut cx = Cx::new_root();
        let root = cx.add(
            container()
                .theme(Theme::dark())
                .child(interactive(rest_pane()).hover(|s| s.background(Token::Accent9))),
        );
        let layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
        let mut router = EventRouter::new();

        let rest = rect_colors(&cx, root);
        assert!(rest.contains(&REST_BG), "rest state paints the base bg");

        router.route(
            &mut cx,
            &layout,
            Event::pointer_move(Point::new(50.0, 50.0)),
        );
        let hovered = rect_colors(&cx, root);
        assert_ne!(
            rest, hovered,
            "class hover overlay must repaint on pointer enter"
        );
        assert!(
            !hovered.contains(&REST_BG),
            "hover slice must override the base bg, got {hovered:?}"
        );

        router.route(
            &mut cx,
            &layout,
            Event::pointer_move(Point::new(300.0, 300.0)),
        );
        assert_eq!(
            rect_colors(&cx, root),
            rest,
            "pointer leave must restore the rest state"
        );
    });
}

#[test]
fn interactive_pressed_overlay_paints_on_pointer_down() {
    Owner::scope("interactive-pressed-paints", || {
        let mut cx = Cx::new_root();
        let root = cx.add(
            interactive(rest_pane())
                .hover(|s| s.background(HOVER_BG))
                .pressed(|s| s.background(PRESSED_BG)),
        );
        let layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
        let mut router = EventRouter::new();

        router.route(
            &mut cx,
            &layout,
            Event::pointer_move(Point::new(50.0, 50.0)),
        );
        assert!(rect_colors(&cx, root).contains(&HOVER_BG));

        router.route(&mut cx, &layout, pointer_down_event(50.0, 50.0));
        let colors = rect_colors(&cx, root);
        assert!(
            colors.contains(&PRESSED_BG),
            "pressed slice must paint on pointer down, got {colors:?}"
        );
        assert!(
            !colors.contains(&HOVER_BG),
            "pressed slice must supersede the hover slice"
        );
    });
}

#[test]
fn interactive_hover_does_not_stop_ancestor_hover() {
    Owner::scope("interactive-non-propagating", || {
        let mut cx = Cx::new_root();
        let inner = interactive(rest_pane()).hover(|s| s.background(HOVER_BG));
        let root = cx.add(
            interactive(
                container()
                    .layout(LayoutStyle::block().fixed_size(200.0, 200.0))
                    .background(REST_BG)
                    .child(inner),
            )
            .hover(|s| s.background(HOVER_BG)),
        );
        let layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
        let mut router = EventRouter::new();
        router.route(
            &mut cx,
            &layout,
            Event::pointer_move(Point::new(50.0, 50.0)),
        );
        let count = rect_colors(&cx, root)
            .iter()
            .filter(|c| **c == HOVER_BG)
            .count();
        assert_eq!(count, 2, "both nested interactive nodes must paint hover");
    });
}

#[test]
fn interactive_disabled_signal_suppresses_hover() {
    Owner::scope("interactive-disabled-inert", || {
        let mut cx = Cx::new_root();
        let disabled = signal(true);
        let root = cx.add(
            interactive(rest_pane())
                .disabled(disabled)
                .hover(|s| s.background(HOVER_BG)),
        );
        let layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
        let mut router = EventRouter::new();
        router.route(
            &mut cx,
            &layout,
            Event::pointer_move(Point::new(50.0, 50.0)),
        );
        assert!(
            !rect_colors(&cx, root).contains(&HOVER_BG),
            "disabled interactive must not paint hover"
        );
    });
}
