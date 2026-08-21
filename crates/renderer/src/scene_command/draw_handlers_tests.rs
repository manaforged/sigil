use super::fit_image;
use sigil_hal::{Point, Rect};
use sigil_kernel::ObjectFit;

// A 40x20 (2:1) image mapped into a 100x100 box.
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
    // Fit-to-width: 100 wide, 50 tall, centered vertically at y=25.
    let (src, out) = fit_image(ObjectFit::Contain, 40.0, 20.0, dest());
    assert_eq!((src.width, src.height), (40.0, 20.0), "full source sampled");
    assert_eq!((out.width, out.height), (100.0, 50.0));
    assert_eq!((out.origin.x, out.origin.y), (0.0, 25.0));
}

#[test]
fn cover_crops_source_and_fills_dest() {
    // Cover a square with a 2:1 image: scale to height, crop width.
    // Visible source width = 20 (of 40), centered at x=10.
    let (src, out) = fit_image(ObjectFit::Cover, 40.0, 20.0, dest());
    assert_eq!(
        (out.width, out.height),
        (100.0, 100.0),
        "dest fully covered"
    );
    assert_eq!((src.width, src.height), (20.0, 20.0));
    assert_eq!((src.origin.x, src.origin.y), (10.0, 0.0));
}
