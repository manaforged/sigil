use super::*;

#[test]
fn justify_start_keeps_items_packed_at_main_start() {
    let mut f = LayoutForest::new();
    let parent = f.insert(
        None,
        StyleData {
            display: Display::Flex,
            justify_content: Some(JustifyContent::Start),
            size: StyleSize::new(Dimension::Px(300.0), Dimension::Px(50.0)),
            ..StyleData::default()
        },
    );
    let a = add_child(&mut f, parent, fixed(50.0, 20.0));
    let s = run(&f, parent, 400.0, 400.0);
    assert!(near(s.boxed(a).unwrap().x, 0.0));
}

#[test]
fn justify_end_packs_items_at_main_end() {
    let mut f = LayoutForest::new();
    let parent = f.insert(
        None,
        StyleData {
            display: Display::Flex,
            justify_content: Some(JustifyContent::End),
            size: StyleSize::new(Dimension::Px(300.0), Dimension::Px(50.0)),
            ..StyleData::default()
        },
    );
    let a = add_child(&mut f, parent, fixed(60.0, 20.0));
    let s = run(&f, parent, 400.0, 400.0);
    let ba = s.boxed(a).unwrap();
    assert!(near(ba.x, 240.0), "ba.x={}", ba.x);
}

#[test]
fn justify_center_centers_items_on_main_axis() {
    let mut f = LayoutForest::new();
    let parent = f.insert(
        None,
        StyleData {
            display: Display::Flex,
            justify_content: Some(JustifyContent::Center),
            size: StyleSize::new(Dimension::Px(300.0), Dimension::Px(50.0)),
            ..StyleData::default()
        },
    );
    let a = add_child(&mut f, parent, fixed(100.0, 20.0));
    let s = run(&f, parent, 400.0, 400.0);
    assert!(near(s.boxed(a).unwrap().x, 100.0));
}

#[test]
fn justify_space_between_distributes_gaps_between_items() {
    let mut f = LayoutForest::new();
    let parent = f.insert(
        None,
        StyleData {
            display: Display::Flex,
            justify_content: Some(JustifyContent::SpaceBetween),
            size: StyleSize::new(Dimension::Px(300.0), Dimension::Px(50.0)),
            ..StyleData::default()
        },
    );
    let a = add_child(&mut f, parent, fixed(50.0, 20.0));
    let b = add_child(&mut f, parent, fixed(50.0, 20.0));
    let s = run(&f, parent, 400.0, 400.0);
    let ba = s.boxed(a).unwrap();
    let bb = s.boxed(b).unwrap();
    assert!(near(ba.x, 0.0));
    assert!(near(bb.x, 250.0), "bb.x={}", bb.x);
}

#[test]
fn justify_space_around_pads_edges_half() {
    let mut f = LayoutForest::new();
    let parent = f.insert(
        None,
        StyleData {
            display: Display::Flex,
            justify_content: Some(JustifyContent::SpaceAround),
            size: StyleSize::new(Dimension::Px(300.0), Dimension::Px(50.0)),
            ..StyleData::default()
        },
    );
    let a = add_child(&mut f, parent, fixed(50.0, 20.0));
    let b = add_child(&mut f, parent, fixed(50.0, 20.0));
    let s = run(&f, parent, 400.0, 400.0);
    let ba = s.boxed(a).unwrap();
    let bb = s.boxed(b).unwrap();
    assert!(near(ba.x, 50.0), "ba.x={}", ba.x);
    assert!(near(bb.x, 200.0), "bb.x={}", bb.x);
}

#[test]
fn justify_space_evenly_pads_all_gaps_equally() {
    let mut f = LayoutForest::new();
    let parent = f.insert(
        None,
        StyleData {
            display: Display::Flex,
            justify_content: Some(JustifyContent::SpaceEvenly),
            size: StyleSize::new(Dimension::Px(310.0), Dimension::Px(50.0)),
            ..StyleData::default()
        },
    );
    let a = add_child(&mut f, parent, fixed(50.0, 20.0));
    let b = add_child(&mut f, parent, fixed(50.0, 20.0));
    let s = run(&f, parent, 400.0, 400.0);
    assert!(near(s.boxed(a).unwrap().x, 70.0));
    assert!(near(s.boxed(b).unwrap().x, 190.0));
}

#[test]
fn align_items_start_keeps_items_at_cross_start() {
    let mut f = LayoutForest::new();
    let parent = f.insert(
        None,
        StyleData {
            display: Display::Flex,
            align_items: Some(AlignItems::Start),
            size: StyleSize::new(Dimension::Px(200.0), Dimension::Px(100.0)),
            ..StyleData::default()
        },
    );
    let a = add_child(&mut f, parent, fixed(40.0, 30.0));
    let s = run(&f, parent, 400.0, 400.0);
    assert!(near(s.boxed(a).unwrap().y, 0.0));
}

#[test]
fn align_items_end_pins_items_to_cross_end() {
    let mut f = LayoutForest::new();
    let parent = f.insert(
        None,
        StyleData {
            display: Display::Flex,
            align_items: Some(AlignItems::End),
            size: StyleSize::new(Dimension::Px(200.0), Dimension::Px(100.0)),
            ..StyleData::default()
        },
    );
    let a = add_child(&mut f, parent, fixed(40.0, 30.0));
    let s = run(&f, parent, 400.0, 400.0);
    assert!(near(s.boxed(a).unwrap().y, 70.0));
}

#[test]
fn align_items_center_centers_items_cross_axis() {
    let mut f = LayoutForest::new();
    let parent = f.insert(
        None,
        StyleData {
            display: Display::Flex,
            align_items: Some(AlignItems::Center),
            size: StyleSize::new(Dimension::Px(200.0), Dimension::Px(100.0)),
            ..StyleData::default()
        },
    );
    let a = add_child(&mut f, parent, fixed(40.0, 30.0));
    let s = run(&f, parent, 400.0, 400.0);
    assert!(near(s.boxed(a).unwrap().y, 35.0));
}

#[test]
fn align_items_stretch_fills_cross_when_cross_size_auto() {
    let mut f = LayoutForest::new();
    let parent = f.insert(
        None,
        StyleData {
            display: Display::Flex,
            align_items: Some(AlignItems::Stretch),
            size: StyleSize::new(Dimension::Px(200.0), Dimension::Px(100.0)),
            ..StyleData::default()
        },
    );
    let a_style = StyleData {
        size: StyleSize::new(Dimension::Px(40.0), Dimension::Auto),
        ..StyleData::default()
    };
    let a = add_child(&mut f, parent, a_style);
    let s = run(&f, parent, 400.0, 400.0);
    let ba = s.boxed(a).unwrap();
    assert!(near(ba.h, 100.0), "ba.h={}", ba.h);
}

#[test]
fn align_self_overrides_container_align_items() {
    let mut f = LayoutForest::new();
    let parent = f.insert(
        None,
        StyleData {
            display: Display::Flex,
            align_items: Some(AlignItems::Start),
            size: StyleSize::new(Dimension::Px(200.0), Dimension::Px(100.0)),
            ..StyleData::default()
        },
    );
    let a_style = StyleData {
        size: StyleSize::new(Dimension::Px(40.0), Dimension::Px(30.0)),
        align_self: Some(AlignSelf::End),
        ..StyleData::default()
    };
    let a = add_child(&mut f, parent, a_style);
    let s = run(&f, parent, 400.0, 400.0);
    assert!(near(s.boxed(a).unwrap().y, 70.0));
}
