use super::*;

#[test]
fn border_box_width_includes_padding() {
    let mut f = LayoutForest::new();
    let style = StyleData {
        size: StyleSize::new(Dimension::Px(100.0), Dimension::Px(50.0)),
        padding: Rect::splat(LengthPercentage::Px(10.0)),
        box_sizing: BoxSizing::BorderBox,
        ..StyleData::default()
    };
    let root = f.insert(None, style);
    let s = run(&f, root, 400.0, 300.0);
    let b = s.boxed(root).unwrap();
    assert!(near(b.w, 100.0));
    assert!(near(b.h, 50.0));
}

#[test]
fn content_box_width_adds_padding() {
    let mut f = LayoutForest::new();
    let style = StyleData {
        size: StyleSize::new(Dimension::Px(100.0), Dimension::Px(50.0)),
        padding: Rect::splat(LengthPercentage::Px(10.0)),
        box_sizing: BoxSizing::ContentBox,
        ..StyleData::default()
    };
    let root = f.insert(None, style);
    let s = run(&f, root, 400.0, 300.0);
    let b = s.boxed(root).unwrap();
    assert!(near(b.w, 120.0));
    assert!(near(b.h, 70.0));
}

#[test]
fn min_width_floors_below_preferred() {
    let mut f = LayoutForest::new();
    let style = StyleData {
        size: StyleSize::new(Dimension::Px(50.0), Dimension::Px(50.0)),
        min_size: StyleSize::new(Dimension::Px(100.0), Dimension::Auto),
        ..StyleData::default()
    };
    let root = f.insert(None, style);
    let s = run(&f, root, 400.0, 300.0);
    let b = s.boxed(root).unwrap();
    assert!(near(b.w, 100.0));
}

#[test]
fn max_height_caps_above_preferred() {
    let mut f = LayoutForest::new();
    let style = StyleData {
        size: StyleSize::new(Dimension::Px(50.0), Dimension::Px(500.0)),
        max_size: StyleSize::new(Dimension::Auto, Dimension::Px(120.0)),
        ..StyleData::default()
    };
    let root = f.insert(None, style);
    let s = run(&f, root, 400.0, 300.0);
    let b = s.boxed(root).unwrap();
    assert!(near(b.h, 120.0));
}

#[test]
fn percent_width_resolves_against_containing_block() {
    let mut f = LayoutForest::new();
    let parent_style = StyleData {
        size: StyleSize::new(Dimension::Px(400.0), Dimension::Px(200.0)),
        ..StyleData::default()
    };
    let child_style = StyleData {
        size: StyleSize::new(Dimension::Percent(0.5), Dimension::Px(30.0)),
        ..StyleData::default()
    };
    let parent = f.insert(None, parent_style);
    let child = f.insert(None, child_style);
    f.attach(parent, child).unwrap();
    let s = run(&f, parent, 1000.0, 800.0);
    let bc = s.boxed(child).unwrap();
    assert!(near(bc.w, 200.0));
}
