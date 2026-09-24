#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use super::*;

#[test]
fn fills_a_triangle_in_color() {
    let blue = Rgba8 {
        r: 40,
        g: 90,
        b: 230,
        a: 255,
    };
    let node = path(32, 32)
        .move_to(16.0, 2.0)
        .line_to(30.0, 30.0)
        .line_to(2.0, 30.0)
        .close()
        .fill(blue);
    let source = node
        .rasterize(&Theme::light())
        .expect("triangle rasterizes");
    assert_eq!(source.width(), 32);
    assert_eq!(source.height(), 32);
    let again = path(32, 32)
        .move_to(16.0, 2.0)
        .line_to(30.0, 30.0)
        .line_to(2.0, 30.0)
        .close()
        .fill(blue)
        .rasterize(&Theme::light())
        .unwrap();
    assert_eq!(
        source.id(),
        again.id(),
        "same commands reuse the CPU raster"
    );

    let rgba = node.rasterize(&Theme::light()).unwrap();
    assert_eq!(rgba.width(), 32);
    let mut pixmap = Pixmap::new(32, 32).unwrap();
    let ts_path = build_ts_path(&node.cmds).unwrap();
    let mut paint = Paint::default();
    paint.set_color(to_ts_color(blue));
    paint.anti_alias = true;
    pixmap.fill_path(
        &ts_path,
        &paint,
        FillRule::Winding,
        Transform::identity(),
        None,
    );
    let center = pixmap.pixel(16, 20).unwrap().demultiply();
    assert!(
        center.alpha() > 200 && center.blue() > center.red(),
        "triangle interior must be opaque blue, got {center:?}"
    );
}

#[test]
fn empty_path_yields_no_image() {
    assert!(
        path(16, 16)
            .fill(Rgba8 {
                r: 0,
                g: 0,
                b: 0,
                a: 255
            })
            .rasterize(&Theme::light())
            .is_none()
    );
}

#[test]
fn fit_rasters_two_x_and_pins_logical_box() {
    let node = path(16, 16)
        .move_to(2.0, 2.0)
        .line_to(14.0, 14.0)
        .stroke(
            Rgba8 {
                r: 255,
                g: 255,
                b: 255,
                a: 255,
            },
            1.35,
        )
        .fit();
    let source = node.rasterize(&Theme::light()).expect("stroke rasterizes");
    assert_eq!(source.width(), 32, "2x pixmap");
    assert_eq!(source.height(), 32);
    assert!(node.layout.is_some(), "fit pins a logical box");
}

#[test]
fn intern_key_distinguishes_gradient_fills() {
    let red = Rgba8 {
        r: 255,
        g: 0,
        b: 0,
        a: 255,
    };
    let blue = Rgba8 {
        r: 0,
        g: 0,
        b: 255,
        a: 255,
    };
    let a = path(16, 16)
        .move_to(0.0, 0.0)
        .line_to(16.0, 16.0)
        .fill_linear((0.0, 0.0), (16.0, 0.0), vec![(0.0, red), (1.0, blue)]);
    let b = path(16, 16)
        .move_to(0.0, 0.0)
        .line_to(16.0, 16.0)
        .fill_linear((0.0, 0.0), (16.0, 0.0), vec![(0.0, blue), (1.0, red)]);
    assert_ne!(
        a.intern_key(a.fill.as_ref(), None),
        b.intern_key(b.fill.as_ref(), None),
        "different gradient stops must not share a raster"
    );
}
