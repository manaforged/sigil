#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]

use sigil::geometry::{Point, Rect};
use sigil::prelude::*;
use sigil::scene::{CornerRadii, SceneRectStyle};
use sigil::style::{NumericAtomKind, Transform2D};
use sigil::widgets::canvas;
use sigil_kittest::Harness;

const RED: Rgba8 = Rgba8 {
    r: 220,
    g: 30,
    b: 30,
    a: 255,
};

fn zoomed(scale: f32) -> NodeBuilder {
    container()
        .bg(Token::Surface)
        .layout(LayoutStyle::block().width(400.0).height(200.0))
        .child(
            container()
                .layout(LayoutStyle::block().width(100.0).height(50.0))
                .transform(Transform2D::IDENTITY.origin(0.0, 0.0))
                .animate(NumericAtomKind::ScaleX, signal(scale))
                .animate(NumericAtomKind::ScaleY, signal(scale))
                .child(
                    container().bg(Token::Accent9).layout(
                        LayoutStyle::block()
                            .width(20.0)
                            .height(20.0)
                            .margin_left(10.25),
                    ),
                ),
        )
}

fn edge_width(image: &image::RgbaImage, y: u32, x0: u32) -> usize {
    let outside = image.get_pixel(x0 - 12, y).0;
    let inside = image.get_pixel(x0 + 12, y).0;
    let far = |a: [u8; 4], b: [u8; 4]| a.iter().zip(b).take(3).any(|(p, q)| p.abs_diff(q) > 12);
    (x0 - 12..x0 + 12)
        .filter(|&x| {
            let px = image.get_pixel(x, y).0;
            far(px, outside) && far(px, inside)
        })
        .count()
}

#[test]
#[ignore = "needs a wgpu adapter"]
fn zoomed_content_keeps_sharp_edges() {
    let mut ui = Harness::new(|| zoomed(4.0)).pixels(400, 200);
    ui.settle();
    let image = ui.image().unwrap().expect("gpu");
    let width = edge_width(&image, 40, 41);
    assert!(width <= 2, "the edge blurs over {width} px at zoom 4");
}

#[test]
#[ignore = "needs a wgpu adapter"]
fn a_canvas_draws_what_its_closure_records() {
    let mut ui = Harness::new(|| {
        canvas(|scene, rect: Rect| {
            let inner = Rect::new(
                Point::new(rect.origin.x + 10.0, rect.origin.y + 10.0),
                30.0,
                20.0,
            );
            scene.draw_rect(inner, SceneRectStyle::new(RED, CornerRadii::ZERO, u16::MAX));
        })
        .layout(LayoutStyle::block().width(100.0).height(60.0))
    })
    .pixels(100, 60);
    ui.settle();
    let image = ui.image().unwrap().expect("gpu");
    let [r, g, b, _] = image.get_pixel(25, 20).0;
    assert!(r > 180 && g < 80 && b < 80, "pixel was {r},{g},{b}");
    let [outer_r, outer_g, _, _] = image.get_pixel(80, 50).0;
    assert!(
        !(outer_r > 180 && outer_g < 80),
        "the canvas drew outside its recorded rect"
    );
}
