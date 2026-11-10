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
        (0.0, 1.0, Length::Auto)
    );
    assert_eq!((s.justify_content, s.align_content), (None, None));
    assert_eq!((s.align_items, s.align_self), (None, None));
    assert_eq!(s.overflow, Axes::splat(Overflow::Visible));
    assert_eq!(s.inset, Edges::splat(Length::Auto));
    assert_eq!(s.size, Axes::splat(Length::Auto));
}

#[test]
fn size_and_rect_splat_fill_every_field() {
    let r: Edges<f32> = Edges::splat(7.0);
    assert_eq!([r.left, r.right, r.top, r.bottom], [7.0; 4]);
    let s: Axes<i32> = Axes::splat(3);
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
