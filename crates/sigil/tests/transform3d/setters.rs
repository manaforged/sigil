use std::cell::Cell;
use std::rc::Rc;

use sigil::geometry::Point;
use sigil::prelude::*;
use sigil_kittest::Harness;

use super::{
    CENTRE, DISTANCE, INK, SCALE, SIDE, VIEW, assert_corners, coverage, css_quad, frame_of,
};

fn stage(card: impl IntoNode + 'static) -> NodeBuilder {
    let pad = CENTRE - SIDE / 2.0;
    container()
        .layout(
            LayoutStyle::block()
                .fixed_size(VIEW as f32, VIEW as f32)
                .padding(Space::raw(pad, "test fixture")),
        )
        .perspective(DISTANCE)
        .child(card)
}

fn painted(degrees: f32, hidden: bool) -> impl Fn() -> NodeBuilder {
    move || {
        let card = container()
            .layout(LayoutStyle::block().fixed_size(SIDE, SIDE))
            .background(INK)
            .rotate_y(degrees);
        stage(if hidden { card.backface_hidden() } else { card })
    }
}

#[test]
fn a_card_turned_by_the_rotate_y_setter_lands_on_the_css_corners() {
    let mut cpu = Harness::new(painted(45.0, false))
        .pixels(VIEW, VIEW)
        .scale(SCALE);
    assert_corners(&frame_of(&mut cpu), "setter pixels");
    let mut gpu = Harness::new(painted(45.0, false)).gpu(VIEW, VIEW, SCALE);
    assert_corners(&frame_of(&mut gpu), "setter gpu");
}

#[test]
fn a_card_turned_to_its_back_by_the_setters_draws_nothing_when_backface_hidden() {
    for (hidden, expect) in [(true, false), (false, true)] {
        let mut ui = Harness::new(painted(180.0, hidden))
            .pixels(VIEW, VIEW)
            .scale(SCALE);
        let found = coverage(&frame_of(&mut ui), Point::new(CENTRE, CENTRE));
        assert_eq!(
            found >= 0.9,
            expect,
            "rotate_y(180) backface_hidden {hidden}: centre ink {found:.2}"
        );
    }
}

#[test]
fn a_turned_card_hits_inside_its_projected_quad_and_reports_its_bounds() {
    let clicks = Rc::new(Cell::new(0));
    let seen = clicks.clone();
    let mut ui = Harness::new(move || {
        let seen = seen.clone();
        stage(
            button("card")
                .layout(LayoutStyle::block().fixed_size(SIDE, SIDE))
                .rotate_y(45.0)
                .activate(move || seen.set(seen.get() + 1)),
        )
    })
    .pixels(VIEW, VIEW);
    ui.click(&Point::new(CENTRE + 40.0, CENTRE));
    assert_eq!(
        clicks.get(),
        0,
        "(C+40, C) lies in the flat box but outside the turned card"
    );
    ui.click(&Point::new(CENTRE - 36.0, CENTRE - 52.0));
    assert_eq!(
        clicks.get(),
        1,
        "(C-36, C-52) lies above the flat box but inside the turned card"
    );
    let want = extent(&css_quad(std::f32::consts::PI / 4.0));
    let bounds = ui
        .get_by_role(Role::Button, "card")
        .bounds()
        .expect("a11y bounds");
    let found = [
        bounds.origin.x,
        bounds.origin.y,
        bounds.origin.x + bounds.width,
        bounds.origin.y + bounds.height,
    ];
    println!("a11y bounds {found:?}, projected quad bounds {want:?}");
    for (got, edge) in found.iter().zip(want) {
        assert!(
            (got - edge).abs() <= 1.0,
            "a11y bounds {found:?} match the quad bounds {want:?}"
        );
    }
}

fn extent(quad: &[Point; 4]) -> [f32; 4] {
    let (x0, x1) = quad
        .iter()
        .fold((f32::MAX, f32::MIN), |(a, b), p| (a.min(p.x), b.max(p.x)));
    let (y0, y1) = quad
        .iter()
        .fold((f32::MAX, f32::MIN), |(a, b), p| (a.min(p.y), b.max(p.y)));
    [x0, y0, x1, y1]
}

fn near_button() -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::block()
                .fixed_size(300.0, 300.0)
                .padding_x(Space::raw(50.0, "test fixture"))
                .padding_y(Space::raw(100.0, "test fixture")),
        )
        .background(Color {
            r: 255,
            g: 255,
            b: 255,
            a: 255,
        })
        .perspective(80.0)
        .child(
            button("near")
                .layout(LayoutStyle::block().fixed_size(200.0, 100.0))
                .background(Color {
                    r: 255,
                    g: 0,
                    b: 0,
                    a: 255,
                })
                .rotate_y(60.0),
        )
}

fn ink_box(frame: &image::RgbaImage) -> [f32; 4] {
    let mut found = [f32::MAX, f32::MAX, f32::MIN, f32::MIN];
    for (x, y, pixel) in frame.enumerate_pixels() {
        if pixel.0[1] < 128 {
            let (x, y) = (x as f32, y as f32);
            found = [
                found[0].min(x),
                found[1].min(y),
                found[2].max(x + 1.0),
                found[3].max(y + 1.0),
            ];
        }
    }
    found
}

#[test]
fn a_card_with_an_edge_behind_the_viewer_reports_the_bounds_it_draws() {
    let mut ui = Harness::new(near_button).unaudited().pixels(300, 300);
    let drawn = ink_box(&frame_of(&mut ui));
    let bounds = ui
        .get_by_role(Role::Button, "near")
        .bounds()
        .expect("a11y bounds");
    let found = [
        bounds.origin.x,
        bounds.origin.y,
        bounds.origin.x + bounds.width,
        bounds.origin.y + bounds.height,
    ];
    println!("a11y bounds {found:?}, drawn ink {drawn:?}");
    assert!(
        found.iter().all(|value| value.is_finite()),
        "finite a11y bounds {found:?}"
    );
    assert!(drawn[0] < drawn[2], "the near card draws ink");
    for (got, want) in found.iter().zip(drawn) {
        let got = got.clamp(0.0, 300.0);
        assert!(
            (got - want).abs() <= 3.0,
            "a11y bounds {found:?} in the view match the drawn ink {drawn:?}"
        );
    }
}
