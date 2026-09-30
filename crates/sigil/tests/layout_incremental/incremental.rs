use super::*;

#[test]
fn hidden_reveal_materializes_never_synced_subtree() {
    Runtime::scope("hidden-reveal", || {
        let mut cx = Cx::new_root();
        let hidden_flag = signal(false);
        let wrapper = container()
            .layout(LayoutStyle::flex_column())
            .visible_when(hidden_flag)
            .child(
                container()
                    .layout(LayoutStyle::block().fixed_size(40.0, 20.0))
                    .child(text("reveal me")),
            );
        let root = cx.add(
            container()
                .layout(LayoutStyle::flex_column())
                .child(wrapper),
        );
        let node_wrapper = cx.get(root).unwrap().children().first().copied().unwrap();
        let panel = cx
            .get(node_wrapper)
            .unwrap()
            .children()
            .first()
            .copied()
            .unwrap();
        let grandchild = cx.get(panel).unwrap().children().first().copied().unwrap();

        let mut engine = stub_engine();
        engine
            .compute(&mut cx, root, AVAILABLE)
            .expect("initial compute with hidden wrapper succeeds");

        hidden_flag.set(true);
        engine
            .compute(&mut cx, root, AVAILABLE)
            .expect("reveal compute must materialize the hidden subtree");
        assert!(layout_result_exists(&cx, panel), "panel mirrored on reveal");
        assert!(
            layout_result_exists(&cx, grandchild),
            "grandchild mirrored on reveal"
        );
        assert!(
            layout_rect(&cx, panel).width > 0.0,
            "revealed panel laid out"
        );
    });
}

#[test]
fn incremental_compute_single_leaf_mutation_preserves_siblings() {
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .layout(LayoutStyle::flex_row().fixed_size(160.0, 40.0))
            .child(container().layout(ordered_fixed(20.0, 10.0, 0)))
            .child(container().layout(ordered_fixed(20.0, 10.0, 2)))
            .child(container().layout(ordered_fixed(20.0, 10.0, 1))),
    );
    let leaf0 = cx.nth_child(root, 0).expect("leaf 0 exists");
    let leaf1 = cx.nth_child(root, 1).expect("leaf 1 exists");
    let leaf2 = cx.nth_child(root, 2).expect("leaf 2 exists");
    let mut engine = stub_engine();

    engine
        .compute(&mut cx, root, AVAILABLE)
        .expect("initial compute succeeds");
    let before0 = layout_result(&cx, leaf0);
    let before2 = layout_result(&cx, leaf2);

    cx.get_mut(leaf1)
        .expect("leaf exists")
        .attach(ordered_fixed(36.0, 10.0, 2));
    engine
        .compute(&mut cx, root, AVAILABLE)
        .expect("incremental compute succeeds");

    assert_eq!(layout_result(&cx, leaf0), before0);
    assert_eq!(layout_result(&cx, leaf2), before2);
    assert_eq!(
        layout_result(&cx, leaf1).rect().width.to_bits(),
        36.0_f32.to_bits()
    );
}

#[test]
fn broadcast_mark_mutated_matches_pre_alpha() {
    let mut cx = Cx::new_root();
    let root = cx.add(container().layout(LayoutStyle::flex_column().fixed_size(120.0, 1000.0)));
    let mut ids = Vec::new();
    for _ in 0..100 {
        ids.push(cx.add_child(
            root,
            container().layout(LayoutStyle::block().fixed_size(20.0, 4.0)),
        ));
    }
    let mut engine = stub_engine();

    engine
        .compute(&mut cx, root, AVAILABLE)
        .expect("initial compute succeeds");
    let before = ids
        .iter()
        .copied()
        .map(|id| (id, layout_result(&cx, id)))
        .collect::<Vec<_>>();

    cx.mark_mutated();
    engine
        .compute(&mut cx, root, AVAILABLE)
        .expect("broadcast compute succeeds");

    for (id, expected) in before {
        assert_eq!(layout_result(&cx, id), expected);
    }
}

#[test]
fn incremental_remove_drops_layout_result() {
    let (mut cx, root, children) = build_many_leaf_row(5);
    let removed = children.get(2).copied().expect("third child exists");
    let mut engine = stub_engine();

    engine
        .compute(&mut cx, root, AVAILABLE)
        .expect("initial compute succeeds");
    assert!(engine.result(removed).is_some());

    assert!(cx.remove(removed));
    engine
        .compute(&mut cx, root, AVAILABLE)
        .expect("incremental remove compute succeeds");

    assert!(cx.get(removed).is_none());
    assert!(engine.result(removed).is_none());
}

#[test]
fn incremental_add_child_attaches_layout_result() {
    let (mut cx, root, _children) = build_many_leaf_row(5);
    let mut engine = stub_engine();

    engine
        .compute(&mut cx, root, AVAILABLE)
        .expect("initial compute succeeds");
    let added = cx.add_child(
        root,
        container().layout(LayoutStyle::block().fixed_size(18.0, 10.0)),
    );
    engine
        .compute(&mut cx, root, AVAILABLE)
        .expect("incremental add compute succeeds");

    assert!(layout_result_exists(&cx, added));
    assert_eq!(
        layout_result(&cx, added).rect().width.to_bits(),
        18.0_f32.to_bits()
    );
}
