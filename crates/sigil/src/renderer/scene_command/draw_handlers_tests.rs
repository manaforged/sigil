use super::fit_image;
use crate::hal::{Point, Rect};
use crate::kernel::ObjectFit;

fn dest() -> Rect {
    Rect::new(Point::new(0.0, 0.0), 100.0, 100.0)
}

#[test]
fn fill_stretches_full_source_to_full_dest() {
    let (src, out) = fit_image(ObjectFit::Fill, 40.0, 20.0, dest());
    assert_eq!((src.width, src.height), (40.0, 20.0));
    assert_eq!((out.width, out.height), (100.0, 100.0));
}

#[test]
fn contain_letterboxes_and_centers() {
    let (src, out) = fit_image(ObjectFit::Contain, 40.0, 20.0, dest());
    assert_eq!((src.width, src.height), (40.0, 20.0), "full source sampled");
    assert_eq!((out.width, out.height), (100.0, 50.0));
    assert_eq!((out.origin.x, out.origin.y), (0.0, 25.0));
}

#[test]
fn cover_crops_source_and_fills_dest() {
    let (src, out) = fit_image(ObjectFit::Cover, 40.0, 20.0, dest());
    assert_eq!(
        (out.width, out.height),
        (100.0, 100.0),
        "dest fully covered"
    );
    assert_eq!((src.width, src.height), (20.0, 20.0));
    assert_eq!((src.origin.x, src.origin.y), (10.0, 0.0));
}
