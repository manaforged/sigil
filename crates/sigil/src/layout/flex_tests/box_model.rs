use super::*;

#[test]
fn padding_offsets_child_origin() {
    let mut f = LayoutForest::new();
    let parent = f.insert(
        None,
        StyleData {
            display: Display::Flex,
            padding: Rect::splat(LengthPercentage::Px(10.0)),
            size: StyleSize::new(Dimension::Px(200.0), Dimension::Px(100.0)),
            ..StyleData::default()
        },
    );
    let a = add_child(&mut f, parent, fixed(40.0, 20.0));
    let s = run(&f, parent, 400.0, 400.0);
    let ba = s.boxed(a).unwrap();
    assert!(near(ba.x, 10.0));
    assert!(near(ba.y, 10.0));
}

#[test]
fn margin_on_item_offsets_main_position() {
    let mut f = LayoutForest::new();
    let parent = f.insert(
        None,
        StyleData {
            display: Display::Flex,
            size: StyleSize::new(Dimension::Px(300.0), Dimension::Px(100.0)),
            ..StyleData::default()
        },
    );
    let a_style = StyleData {
        size: StyleSize::new(Dimension::Px(40.0), Dimension::Px(20.0)),
        margin: Rect {
            left: LengthPercentageAuto::Px(15.0),
            right: LengthPercentageAuto::Px(0.0),
            top: LengthPercentageAuto::Px(0.0),
            bottom: LengthPercentageAuto::Px(0.0),
        },
        ..StyleData::default()
    };
    let a = add_child(&mut f, parent, a_style);
    let s = run(&f, parent, 400.0, 400.0);
    assert!(near(s.boxed(a).unwrap().x, 15.0));
}

#[test]
fn margin_on_a_nested_flex_container_applies_once_not_twice() {
    let mut f = LayoutForest::new();
    let parent = f.insert(
        None,
        StyleData {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            size: StyleSize::new(Dimension::Px(300.0), Dimension::Px(300.0)),
            ..StyleData::default()
        },
    );
    let child_style = StyleData {
        display: Display::Flex,
        flex_direction: FlexDirection::Column,
        size: StyleSize::new(Dimension::Px(100.0), Dimension::Px(50.0)),
        margin: Rect {
            left: LengthPercentageAuto::Px(0.0),
            right: LengthPercentageAuto::Px(0.0),
            top: LengthPercentageAuto::Px(20.0),
            bottom: LengthPercentageAuto::Px(0.0),
        },
        ..StyleData::default()
    };
    let child = add_child(&mut f, parent, child_style);
    let grandchild_style = StyleData {
        size: StyleSize::new(Dimension::Px(10.0), Dimension::Px(10.0)),
        ..StyleData::default()
    };
    let _gc = add_child(&mut f, child, grandchild_style);

    let s = run(&f, parent, 400.0, 400.0);
    let y = s.boxed(child).unwrap().y;
    assert!(
        near(y, 20.0),
        "nested flex container top margin must apply once (expected 20, got {y})"
    );
}

#[test]
fn min_size_floors_main_axis_shrink() {
    let mut f = LayoutForest::new();
    let parent = f.insert(
        None,
        StyleData {
            display: Display::Flex,
            size: StyleSize::new(Dimension::Px(100.0), Dimension::Px(50.0)),
            ..StyleData::default()
        },
    );
    let a_style = StyleData {
        flex_basis: Dimension::Px(80.0),
        flex_shrink: 1.0,
        min_size: StyleSize::new(Dimension::Px(70.0), Dimension::Auto),
        size: StyleSize::new(Dimension::Auto, Dimension::Px(20.0)),
        ..StyleData::default()
    };
    let a = add_child(&mut f, parent, a_style.clone());
    let b = add_child(&mut f, parent, a_style);
    let s = run(&f, parent, 400.0, 400.0);
    let ba = s.boxed(a).unwrap();
    let bb = s.boxed(b).unwrap();
    assert!(ba.w >= 70.0 - TOL);
    assert!(bb.w >= 70.0 - TOL);
}

#[test]
fn max_size_caps_main_axis_grow() {
    let mut f = LayoutForest::new();
    let parent = f.insert(
        None,
        StyleData {
            display: Display::Flex,
            size: StyleSize::new(Dimension::Px(300.0), Dimension::Px(50.0)),
            ..StyleData::default()
        },
    );
    let a_style = StyleData {
        flex_basis: Dimension::Px(0.0),
        flex_grow: 1.0,
        max_size: StyleSize::new(Dimension::Px(80.0), Dimension::Auto),
        size: StyleSize::new(Dimension::Auto, Dimension::Px(20.0)),
        ..StyleData::default()
    };
    let a = add_child(&mut f, parent, a_style);
    let s = run(&f, parent, 400.0, 400.0);
    let ba = s.boxed(a).unwrap();
    assert!(ba.w <= 80.0 + TOL, "ba.w={}", ba.w);
}
