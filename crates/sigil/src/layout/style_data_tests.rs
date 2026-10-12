use super::*;

#[test]
fn default_matches_css_baseline() {
    let s = StyleData::default();
    assert_eq!(
        (
            s.display,
            s.position,
            s.flex_direction,
            s.flex_wrap,
            s.box_sizing
        ),
        (
            Display::Block,
            Position::Relative,
            FlexDirection::Row,
            FlexWrap::NoWrap,
            BoxSizing::ContentBox
        )
    );
    assert_eq!(
        (s.flex_grow, s.flex_shrink, s.flex_basis),
        (0.0, 1.0, Dimension::Auto)
    );
    assert_eq!((s.justify_content, s.align_content), (None, None));
    assert_eq!((s.align_items, s.align_self), (None, None));
    assert_eq!(s.overflow, Size::splat(Overflow::Visible));
    assert_eq!(s.inset, Rect::splat(LengthPercentageAuto::Auto));
    assert_eq!(s.size, Size::splat(Dimension::Auto));
}

#[test]
fn size_and_rect_splat_fill_every_field() {
    let r: Rect<f32> = Rect::splat(7.0);
    assert_eq!([r.left, r.right, r.top, r.bottom], [7.0; 4]);
    let s: Size<i32> = Size::splat(3);
    assert_eq!((s.width, s.height), (3, 3));
}

#[test]
fn style_data_is_cloneable_and_eq_aware() {
    let a = StyleData::default();
    assert_eq!(a, a.clone());
    let c = StyleData {
        flex_grow: 1.0,
        ..a.clone()
    };
    assert_ne!(a, c);
}
