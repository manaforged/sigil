use sigil_hal::{Point, Rect};

use super::physical_rect;

#[test]
fn converts_fractional_logical_clip_to_physical_pixels() {
    let clip = Rect::new(Point::new(2.25, 3.75), 4.5, 5.0);
    assert_eq!(physical_rect(clip, 2.0, (20.0, 20.0)), Some([4, 7, 10, 11]));
}

#[test]
fn clamps_to_the_framebuffer_and_rejects_empty_regions() {
    let partial = Rect::new(Point::new(-5.0, 8.0), 10.0, 10.0);
    assert_eq!(
        physical_rect(partial, 1.0, (12.0, 12.0)),
        Some([0, 8, 5, 4])
    );

    let outside = Rect::new(Point::new(20.0, 20.0), 4.0, 4.0);
    assert_eq!(physical_rect(outside, 1.0, (12.0, 12.0)), None);
}
