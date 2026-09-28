use super::*;

#[test]
fn row_lays_children_along_inline_axis() {
    let mut f = LayoutForest::new();
    let parent = f.insert(
        None,
        StyleData {
            display: Display::Flex,
            size: StyleSize::new(Dimension::Px(300.0), Dimension::Px(100.0)),
            ..StyleData::default()
        },
    );
    let a = add_child(&mut f, parent, fixed(40.0, 30.0));
    let b = add_child(&mut f, parent, fixed(60.0, 30.0));
    let s = run(&f, parent, 400.0, 400.0);
    let ba = s.boxed(a).unwrap();
    let bb = s.boxed(b).unwrap();
    assert!(near(ba.x, 0.0), "ba.x={}", ba.x);
    assert!(near(ba.y, 0.0));
    assert!(near(bb.x, 40.0), "bb.x={}", bb.x);
    assert!(near(bb.y, 0.0));
}

#[test]
fn grow_item_is_not_start_packed() {
    let mut f = LayoutForest::new();
    let parent = f.insert(
        None,
        StyleData {
            display: Display::Flex,
            size: StyleSize::new(Dimension::Px(300.0), Dimension::Px(40.0)),
            ..StyleData::default()
        },
    );
    let a = add_child(&mut f, parent, fixed(40.0, 40.0));
    let mut grown = fixed(40.0, 40.0);
    grown.flex_grow = 1.0;
    let b = add_child(&mut f, parent, grown);
    let s = run(&f, parent, 400.0, 400.0);
    let bb = s.boxed(b).unwrap();
    assert!(
        bb.w > 200.0,
        "grow item must take leftover main space, w={}",
        bb.w
    );
    let _ = a;
}

#[test]
fn column_lays_children_along_block_axis() {
    let mut f = LayoutForest::new();
    let parent = f.insert(
        None,
        StyleData {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            size: StyleSize::new(Dimension::Px(200.0), Dimension::Px(300.0)),
            ..StyleData::default()
        },
    );
    let a = add_child(&mut f, parent, fixed(40.0, 30.0));
    let b = add_child(&mut f, parent, fixed(40.0, 50.0));
    let s = run(&f, parent, 400.0, 400.0);
    let ba = s.boxed(a).unwrap();
    let bb = s.boxed(b).unwrap();
    assert!(near(ba.y, 0.0));
    assert!(near(bb.y, 30.0), "bb.y={}", bb.y);
}

#[test]
fn row_reverse_reverses_main_order() {
    let mut f = LayoutForest::new();
    let parent = f.insert(
        None,
        StyleData {
            display: Display::Flex,
            flex_direction: FlexDirection::RowReverse,
            size: StyleSize::new(Dimension::Px(200.0), Dimension::Px(100.0)),
            ..StyleData::default()
        },
    );
    let a = add_child(&mut f, parent, fixed(40.0, 20.0));
    let b = add_child(&mut f, parent, fixed(60.0, 20.0));
    let s = run(&f, parent, 400.0, 400.0);
    let ba = s.boxed(a).unwrap();
    let bb = s.boxed(b).unwrap();
    assert!(near(bb.x, 0.0), "bb.x={}", bb.x);
    assert!(near(ba.x, 60.0), "ba.x={}", ba.x);
}

#[test]
fn column_reverse_reverses_block_order() {
    let mut f = LayoutForest::new();
    let parent = f.insert(
        None,
        StyleData {
            display: Display::Flex,
            flex_direction: FlexDirection::ColumnReverse,
            size: StyleSize::new(Dimension::Px(100.0), Dimension::Px(200.0)),
            ..StyleData::default()
        },
    );
    let a = add_child(&mut f, parent, fixed(40.0, 30.0));
    let b = add_child(&mut f, parent, fixed(40.0, 50.0));
    let s = run(&f, parent, 400.0, 400.0);
    let ba = s.boxed(a).unwrap();
    let bb = s.boxed(b).unwrap();
    assert!(near(bb.y, 0.0));
    assert!(near(ba.y, 50.0), "ba.y={}", ba.y);
}

#[test]
fn flex_basis_overrides_size_for_main_axis() {
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
        size: StyleSize::new(Dimension::Px(10.0), Dimension::Px(20.0)),
        flex_basis: Dimension::Px(80.0),
        ..StyleData::default()
    };
    let a = add_child(&mut f, parent, a_style);
    let s = run(&f, parent, 400.0, 400.0);
    let ba = s.boxed(a).unwrap();
    assert!(near(ba.w, 80.0), "ba.w={}", ba.w);
}

#[test]
fn flex_grow_distributes_free_space_proportionally() {
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
        flex_basis: Dimension::Px(0.0),
        flex_grow: 1.0,
        size: StyleSize::new(Dimension::Auto, Dimension::Px(20.0)),
        ..StyleData::default()
    };
    let b_style = StyleData {
        flex_basis: Dimension::Px(0.0),
        flex_grow: 2.0,
        size: StyleSize::new(Dimension::Auto, Dimension::Px(20.0)),
        ..StyleData::default()
    };
    let a = add_child(&mut f, parent, a_style);
    let b = add_child(&mut f, parent, b_style);
    let s = run(&f, parent, 400.0, 400.0);
    let ba = s.boxed(a).unwrap();
    let bb = s.boxed(b).unwrap();
    assert!(near(ba.w, 100.0), "ba.w={}", ba.w);
    assert!(near(bb.w, 200.0), "bb.w={}", bb.w);
}

#[test]
fn flex_shrink_distributes_deficit_proportionally() {
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
        size: StyleSize::new(Dimension::Auto, Dimension::Px(20.0)),
        ..StyleData::default()
    };
    let a = add_child(&mut f, parent, a_style.clone());
    let b = add_child(&mut f, parent, a_style);
    let s = run(&f, parent, 400.0, 400.0);
    let ba = s.boxed(a).unwrap();
    let bb = s.boxed(b).unwrap();
    assert!(near(ba.w, 50.0), "ba.w={}", ba.w);
    assert!(near(bb.w, 50.0), "bb.w={}", bb.w);
}

#[test]
fn no_grow_no_shrink_keeps_basis() {
    let mut f = LayoutForest::new();
    let parent = f.insert(
        None,
        StyleData {
            display: Display::Flex,
            size: StyleSize::new(Dimension::Px(400.0), Dimension::Px(50.0)),
            ..StyleData::default()
        },
    );
    let a_style = StyleData {
        flex_basis: Dimension::Px(60.0),
        flex_grow: 0.0,
        flex_shrink: 0.0,
        size: StyleSize::new(Dimension::Auto, Dimension::Px(20.0)),
        ..StyleData::default()
    };
    let a = add_child(&mut f, parent, a_style);
    let s = run(&f, parent, 400.0, 400.0);
    let ba = s.boxed(a).unwrap();
    assert!(near(ba.w, 60.0));
}
