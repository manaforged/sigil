use super::*;

#[test]
fn default_matches_css_baseline() {
    let s = StyleData::default();
    assert_eq!(s.display, Display::Block);
    assert_eq!(s.position, Position::Relative);
    assert_eq!(s.flex_direction, FlexDirection::Row);
    assert_eq!(s.flex_wrap, FlexWrap::NoWrap);
    assert!((s.flex_grow - 0.0).abs() < f32::EPSILON);
    assert!((s.flex_shrink - 1.0).abs() < f32::EPSILON);
    assert_eq!(s.flex_basis, Dimension::Auto);
    assert_eq!(s.justify_content, None);
    assert_eq!(s.align_items, None);
    assert_eq!(s.align_content, None);
    assert_eq!(s.align_self, None);
    assert_eq!(s.overflow.width, Overflow::Visible);
    assert_eq!(s.overflow.height, Overflow::Visible);
    assert_eq!(s.inset.left, LengthPercentageAuto::Auto);
    assert_eq!(s.inset.right, LengthPercentageAuto::Auto);
    assert_eq!(s.inset.top, LengthPercentageAuto::Auto);
    assert_eq!(s.inset.bottom, LengthPercentageAuto::Auto);
    assert_eq!(s.size.width, Dimension::Auto);
    assert_eq!(s.size.height, Dimension::Auto);
    assert_eq!(s.box_sizing, BoxSizing::ContentBox);
}

#[test]
fn size_and_rect_splat_fill_every_field() {
    let r: Rect<f32> = Rect::splat(7.0);
    assert!((r.left - 7.0).abs() < f32::EPSILON);
    assert!((r.right - 7.0).abs() < f32::EPSILON);
    assert!((r.top - 7.0).abs() < f32::EPSILON);
    assert!((r.bottom - 7.0).abs() < f32::EPSILON);
    let s: Size<i32> = Size::splat(3);
    assert_eq!(s.width, 3);
    assert_eq!(s.height, 3);
}

#[test]
fn style_data_is_cloneable_and_eq_aware() {
    let a = StyleData::default();
    let b = a.clone();
    assert_eq!(a, b);
    let mut c = a.clone();
    c.flex_grow = 1.0;
    assert_ne!(a, c);
}
