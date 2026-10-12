use super::*;

#[test]
fn clamp_lets_min_win_and_passes_through_unbounded() {
    assert!(near(clamp(80.0, Some(120.0), Some(100.0)), 120.0));
    assert!(near(clamp(42.0, None, None), 42.0));
}

#[test]
fn resolve_lp_percent_resolves_against_basis() {
    assert!(near(
        resolve_lp(LengthPercentage::Percent(0.25), 200.0),
        50.0
    ));
}

#[test]
fn dim_to_border_box_honors_box_sizing_and_auto() {
    let px = Dimension::Px(100.0);
    assert_eq!(
        dim_to_border_box(px, 0.0, 20.0, BoxSizing::ContentBox),
        Some(120.0)
    );
    assert_eq!(
        dim_to_border_box(px, 0.0, 20.0, BoxSizing::BorderBox),
        Some(100.0)
    );
    assert!(dim_to_border_box(Dimension::Auto, 100.0, 0.0, BoxSizing::ContentBox).is_none());
}

#[test]
fn algo_edges_inline_and_block_sum_match_struct_fields() {
    let e = AlgoEdges {
        left: 1.0,
        right: 2.0,
        top: 3.0,
        bottom: 4.0,
    };
    assert!(near(e.inline(), 3.0));
    assert!(near(e.block(), 7.0));
}

#[test]
fn percent_dimension_resolves_against_container() {
    let item = sized(Dimension::Percent(0.25), Dimension::Px(20.0));
    assert!(near(kids(flex_box(400.0, 100.0), &[item])[0].w, 100.0));
}

#[test]
fn nested_auto_height_columns_probe_once_per_node() {
    let mut f = LayoutForest::new();
    let column = StyleData {
        display: Display::Flex,
        flex_direction: FlexDirection::Column,
        ..sized(Dimension::Px(200.0), Dimension::Auto)
    };
    let root = f.insert(
        None,
        StyleData {
            size: StyleSize::new(Dimension::Px(200.0), Dimension::Px(400.0)),
            ..column.clone()
        },
    );
    let mut parent = root;
    for _ in 0..8 {
        parent = add_child(&mut f, parent, column.clone());
    }
    let leaf = add_child(&mut f, parent, fixed(20.0, 16.0));
    let s = run(&f, root, 200.0, 400.0);
    assert!(
        s.intrinsic_stores() <= 16,
        "intrinsic stores {} (want linear in depth, not 2^depth)",
        s.intrinsic_stores()
    );
    let box_ = s.boxed(leaf).unwrap();
    assert!(near(box_.h, 16.0), "leaf h={}", box_.h);
}

#[test]
fn fill_child_inside_grow_item_does_not_shrink_siblings() {
    let mut f = LayoutForest::new();
    let parent = f.insert(
        None,
        StyleData {
            box_sizing: BoxSizing::BorderBox,
            ..flex_box(272.0, 40.0)
        },
    );
    let fixed_leaf = add_child(&mut f, parent, fixed(56.0, 8.0));
    let wrapper = add_child(
        &mut f,
        parent,
        StyleData {
            display: Display::Flex,
            box_sizing: BoxSizing::BorderBox,
            flex_grow: 1.0,
            min_size: StyleSize::new(Dimension::Px(0.0), Dimension::Px(0.0)),
            ..StyleData::default()
        },
    );
    add_child(
        &mut f,
        wrapper,
        StyleData {
            display: Display::Block,
            box_sizing: BoxSizing::BorderBox,
            ..sized(Dimension::Fill, Dimension::Px(1.0))
        },
    );
    let s = run(&f, parent, 400.0, 400.0);
    let leaf = s.boxed(fixed_leaf).unwrap();
    let wrap = s.boxed(wrapper).unwrap();
    assert!(near(leaf.w, 56.0), "sibling shrank: leaf.w={}", leaf.w);
    assert!(near(wrap.x, 56.0), "wrap.x={}", wrap.x);
    assert!(near(wrap.w, 216.0), "wrap.w={}", wrap.w);
}
