use super::*;

#[test]
fn order_reorders_items_by_ascending_order_field() {
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
        size: StyleSize::new(Dimension::Px(40.0), Dimension::Px(20.0)),
        order: 10,
        ..StyleData::default()
    };
    let b_style = StyleData {
        size: StyleSize::new(Dimension::Px(40.0), Dimension::Px(20.0)),
        order: -5,
        ..StyleData::default()
    };
    let a = add_child(&mut f, parent, a_style);
    let b = add_child(&mut f, parent, b_style);
    let s = run(&f, parent, 400.0, 400.0);
    assert!(near(s.boxed(b).unwrap().x, 0.0));
    assert!(near(s.boxed(a).unwrap().x, 40.0));
}

#[test]
fn display_none_items_are_skipped() {
    let mut f = LayoutForest::new();
    let parent = f.insert(
        None,
        StyleData {
            display: Display::Flex,
            size: StyleSize::new(Dimension::Px(300.0), Dimension::Px(50.0)),
            ..StyleData::default()
        },
    );
    let hidden_style = StyleData {
        size: StyleSize::new(Dimension::Px(40.0), Dimension::Px(20.0)),
        display: Display::None,
        ..StyleData::default()
    };
    let _hidden = add_child(&mut f, parent, hidden_style);
    let visible = add_child(&mut f, parent, fixed(60.0, 20.0));
    let s = run(&f, parent, 400.0, 400.0);
    assert!(near(s.boxed(visible).unwrap().x, 0.0));
}

#[test]
fn absolute_items_are_isolated_from_flex_flow() {
    let mut f = LayoutForest::new();
    let parent = f.insert(
        None,
        StyleData {
            display: Display::Flex,
            size: StyleSize::new(Dimension::Px(300.0), Dimension::Px(100.0)),
            ..StyleData::default()
        },
    );
    let abs_style = StyleData {
        size: StyleSize::new(Dimension::Px(40.0), Dimension::Px(20.0)),
        position: Position::Absolute,
        ..StyleData::default()
    };
    let _abs = add_child(&mut f, parent, abs_style);
    let flow = add_child(&mut f, parent, fixed(60.0, 20.0));
    let s = run(&f, parent, 400.0, 400.0);
    assert!(near(s.boxed(flow).unwrap().x, 0.0));
}

#[test]
fn container_box_picks_up_explicit_size() {
    let mut f = LayoutForest::new();
    let parent = f.insert(
        None,
        StyleData {
            display: Display::Flex,
            size: StyleSize::new(Dimension::Px(220.0), Dimension::Px(70.0)),
            ..StyleData::default()
        },
    );
    let _a = add_child(&mut f, parent, fixed(40.0, 20.0));
    let s = run(&f, parent, 400.0, 400.0);
    let b = s.boxed(parent).unwrap();
    assert!(near(b.w, 220.0));
    assert!(near(b.h, 70.0));
}

#[test]
fn solver_writes_results_through_forest_for_visited_nodes() {
    let mut f = LayoutForest::new();
    let parent = f.insert(None, flex_parent());
    let child = add_child(&mut f, parent, fixed(20.0, 20.0));
    let s = run(&f, parent, 200.0, 100.0);
    assert!(s.boxed(parent).is_some());
    assert!(s.boxed(child).is_some());
}

#[test]
fn empty_flex_container_produces_only_self_box() {
    let mut f = LayoutForest::new();
    let parent = f.insert(
        None,
        StyleData {
            display: Display::Flex,
            size: StyleSize::new(Dimension::Px(100.0), Dimension::Px(50.0)),
            ..StyleData::default()
        },
    );
    let s = run(&f, parent, 200.0, 100.0);
    assert!(s.boxed(parent).is_some());
}
