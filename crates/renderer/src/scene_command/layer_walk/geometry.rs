use sigil_hal::{Point, Rect};

pub(super) const fn local_rect(rect: Rect, parent_origin: Point) -> Rect {
    Rect::new(
        Point::new(
            rect.origin.x - parent_origin.x,
            rect.origin.y - parent_origin.y,
        ),
        rect.width,
        rect.height,
    )
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub(super) fn physical_extent(rect: Rect, scale: f32) -> (u32, u32) {
    let scale = scale.max(0.0);
    (
        (rect.width * scale).ceil().max(0.0) as u32,
        (rect.height * scale).ceil().max(0.0) as u32,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn physical_extent_scales_fractional_logical_bounds_outward() {
        let rect = Rect::new(Point::new(10.0, 20.0), 10.25, 5.25);
        assert_eq!(physical_extent(rect, 2.0), (21, 11));
    }
}
