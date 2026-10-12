use super::*;

fn margin(left: f32, top: f32) -> Rect<LengthPercentageAuto> {
    Rect {
        left: LengthPercentageAuto::Px(left),
        right: LengthPercentageAuto::Px(0.0),
        top: LengthPercentageAuto::Px(top),
        bottom: LengthPercentageAuto::Px(0.0),
    }
}

#[test]
fn padding_offsets_child_origin() {
    let parent = StyleData {
        padding: Rect::splat(LengthPercentage::Px(10.0)),
        ..flex_box(200.0, 100.0)
    };
    let b = kids(parent, &[fixed(40.0, 20.0)])[0];
    assert!(near(b.x, 10.0) && near(b.y, 10.0), "{b:?}");
}

#[test]
fn margin_on_item_offsets_main_position() {
    let item = StyleData {
        margin: margin(15.0, 0.0),
        ..fixed(40.0, 20.0)
    };
    assert!(near(kids(flex_box(300.0, 100.0), &[item])[0].x, 15.0));
}

#[test]
fn margin_on_a_nested_flex_container_applies_once_not_twice() {
    let mut f = LayoutForest::new();
    let parent = f.insert(
        None,
        StyleData {
            flex_direction: FlexDirection::Column,
            ..flex_box(300.0, 300.0)
        },
    );
    let child_style = StyleData {
        flex_direction: FlexDirection::Column,
        margin: margin(0.0, 20.0),
        ..flex_box(100.0, 50.0)
    };
    let child = add_child(&mut f, parent, child_style);
    add_child(&mut f, child, fixed(10.0, 10.0));
    let y = run(&f, parent, 400.0, 400.0).boxed(child).unwrap().y;
    assert!(
        near(y, 20.0),
        "nested flex container top margin must apply once (expected 20, got {y})"
    );
}

#[test]
fn min_size_floors_main_axis_shrink() {
    let item = StyleData {
        flex_basis: Dimension::Px(80.0),
        flex_shrink: 1.0,
        min_size: StyleSize::new(Dimension::Px(70.0), Dimension::Auto),
        ..sized(Dimension::Auto, Dimension::Px(20.0))
    };
    for b in kids(flex_box(100.0, 50.0), &[item.clone(), item]) {
        assert!(b.w >= 70.0 - TOL, "w={}", b.w);
    }
}

#[test]
fn max_size_caps_main_axis_grow() {
    let item = StyleData {
        flex_basis: Dimension::Px(0.0),
        flex_grow: 1.0,
        max_size: StyleSize::new(Dimension::Px(80.0), Dimension::Auto),
        ..sized(Dimension::Auto, Dimension::Px(20.0))
    };
    let b = kids(flex_box(300.0, 50.0), &[item])[0];
    assert!(b.w <= 80.0 + TOL, "w={}", b.w);
}
