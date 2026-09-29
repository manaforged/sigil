use crate::geometry::{Point, Rect};
use crate::kernel::ObjectFit;
use crate::scene::command::image_fit::fit;

fn dest() -> Rect {
    Rect::new(Point::new(0.0, 0.0), 100.0, 100.0)
}

fn source() -> Rect {
    Rect::new(Point::new(0.0, 0.0), 40.0, 20.0)
}

#[test]
fn fill_stretches_full_source_to_full_dest() {
    let (src, out) = fit(ObjectFit::Fill, source(), dest());
    assert_eq!((src.width, src.height), (40.0, 20.0));
    assert_eq!((out.width, out.height), (100.0, 100.0));
}

#[test]
fn contain_letterboxes_and_centers() {
    let (src, out) = fit(ObjectFit::Contain, source(), dest());
    assert_eq!((src.width, src.height), (40.0, 20.0), "full source sampled");
    assert_eq!((out.width, out.height), (100.0, 50.0));
    assert_eq!((out.origin.x, out.origin.y), (0.0, 25.0));
}

#[test]
fn cover_crops_source_and_fills_dest() {
    let (src, out) = fit(ObjectFit::Cover, source(), dest());
    assert_eq!(
        (out.width, out.height),
        (100.0, 100.0),
        "dest fully covered"
    );
    assert_eq!((src.width, src.height), (20.0, 20.0));
    assert_eq!((src.origin.x, src.origin.y), (10.0, 0.0));
}
