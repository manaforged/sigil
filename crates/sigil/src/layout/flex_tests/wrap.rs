use super::*;

#[test]
fn nowrap_keeps_overflowing_items_on_single_line() {
    let mut f = LayoutForest::new();
    let parent = f.insert(
        None,
        StyleData {
            display: Display::Flex,
            flex_wrap: FlexWrap::NoWrap,
            size: StyleSize::new(Dimension::Px(150.0), Dimension::Px(100.0)),
            ..StyleData::default()
        },
    );
    let a_style = StyleData {
        size: StyleSize::new(Dimension::Px(80.0), Dimension::Px(20.0)),
        flex_shrink: 0.0,
        ..StyleData::default()
    };
    let a = add_child(&mut f, parent, a_style.clone());
    let b = add_child(&mut f, parent, a_style.clone());
    let c = add_child(&mut f, parent, a_style);
    let s = run(&f, parent, 400.0, 400.0);
    assert!(near(s.boxed(a).unwrap().y, 0.0));
    assert!(near(s.boxed(b).unwrap().y, 0.0));
    assert!(near(s.boxed(c).unwrap().y, 0.0));
}

#[test]
fn wrap_breaks_overflowing_items_onto_next_line() {
    let mut f = LayoutForest::new();
    let parent = f.insert(
        None,
        StyleData {
            display: Display::Flex,
            flex_wrap: FlexWrap::Wrap,
            size: StyleSize::new(Dimension::Px(150.0), Dimension::Px(200.0)),
            ..StyleData::default()
        },
    );
    let a_style = StyleData {
        size: StyleSize::new(Dimension::Px(80.0), Dimension::Px(30.0)),
        flex_shrink: 0.0,
        flex_grow: 0.0,
        ..StyleData::default()
    };
    let a = add_child(&mut f, parent, a_style.clone());
    let b = add_child(&mut f, parent, a_style.clone());
    let c = add_child(&mut f, parent, a_style);
    let s = run(&f, parent, 400.0, 400.0);
    let ba = s.boxed(a).unwrap();
    let bb = s.boxed(b).unwrap();
    let bc = s.boxed(c).unwrap();
    assert!(near(ba.y, 0.0));
    assert!(near(bb.y, 30.0), "bb.y={}", bb.y);
    assert!(near(bc.y, 60.0), "bc.y={}", bc.y);
}

#[test]
fn wrap_reverse_orders_lines_from_cross_end() {
    let mut f = LayoutForest::new();
    let parent = f.insert(
        None,
        StyleData {
            display: Display::Flex,
            flex_wrap: FlexWrap::WrapReverse,
            align_content: Some(AlignContent::Start),
            size: StyleSize::new(Dimension::Px(150.0), Dimension::Px(200.0)),
            ..StyleData::default()
        },
    );
    let a_style = StyleData {
        size: StyleSize::new(Dimension::Px(80.0), Dimension::Px(30.0)),
        flex_shrink: 0.0,
        flex_grow: 0.0,
        ..StyleData::default()
    };
    let a = add_child(&mut f, parent, a_style.clone());
    let b = add_child(&mut f, parent, a_style);
    let s = run(&f, parent, 400.0, 400.0);
    let ba = s.boxed(a).unwrap();
    let bb = s.boxed(b).unwrap();
    assert!(near(bb.y, 0.0), "bb.y={}", bb.y);
    assert!(near(ba.y, 30.0), "ba.y={}", ba.y);
}

#[test]
fn align_content_center_centers_lines_cross_axis() {
    let mut f = LayoutForest::new();
    let parent = f.insert(
        None,
        StyleData {
            display: Display::Flex,
            flex_wrap: FlexWrap::Wrap,
            align_content: Some(AlignContent::Center),
            size: StyleSize::new(Dimension::Px(150.0), Dimension::Px(200.0)),
            ..StyleData::default()
        },
    );
    let a_style = StyleData {
        size: StyleSize::new(Dimension::Px(80.0), Dimension::Px(30.0)),
        flex_shrink: 0.0,
        flex_grow: 0.0,
        ..StyleData::default()
    };
    let a = add_child(&mut f, parent, a_style.clone());
    let b = add_child(&mut f, parent, a_style);
    let s = run(&f, parent, 400.0, 400.0);
    assert!(near(s.boxed(a).unwrap().y, 70.0));
    assert!(near(s.boxed(b).unwrap().y, 100.0));
}

#[test]
fn align_content_end_packs_lines_at_cross_end() {
    let mut f = LayoutForest::new();
    let parent = f.insert(
        None,
        StyleData {
            display: Display::Flex,
            flex_wrap: FlexWrap::Wrap,
            align_content: Some(AlignContent::End),
            size: StyleSize::new(Dimension::Px(150.0), Dimension::Px(200.0)),
            ..StyleData::default()
        },
    );
    let a_style = StyleData {
        size: StyleSize::new(Dimension::Px(80.0), Dimension::Px(30.0)),
        flex_shrink: 0.0,
        flex_grow: 0.0,
        ..StyleData::default()
    };
    let a = add_child(&mut f, parent, a_style.clone());
    let b = add_child(&mut f, parent, a_style);
    let s = run(&f, parent, 400.0, 400.0);
    assert!(near(s.boxed(a).unwrap().y, 140.0));
    assert!(near(s.boxed(b).unwrap().y, 170.0));
}

#[test]
fn gap_inserts_main_axis_gutter_between_items() {
    let mut f = LayoutForest::new();
    let parent = f.insert(
        None,
        StyleData {
            display: Display::Flex,
            gap: StyleSize::new(LengthPercentage::Px(8.0), LengthPercentage::Px(0.0)),
            size: StyleSize::new(Dimension::Px(300.0), Dimension::Px(100.0)),
            ..StyleData::default()
        },
    );
    let a = add_child(&mut f, parent, fixed(40.0, 20.0));
    let b = add_child(&mut f, parent, fixed(40.0, 20.0));
    let s = run(&f, parent, 400.0, 400.0);
    assert!(near(s.boxed(a).unwrap().x, 0.0));
    assert!(
        near(s.boxed(b).unwrap().x, 48.0),
        "b.x={}",
        s.boxed(b).unwrap().x
    );
}
