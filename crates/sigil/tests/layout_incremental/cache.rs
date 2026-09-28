use super::*;

#[test]
fn cached_compute_returns_immediately_without_recomputing() {
    let (mut cx, root, _child) = build_tree();
    let mut engine = stub_engine();

    engine
        .compute(&mut cx, root, AVAILABLE)
        .expect("initial compute succeeds");
    assert!(layout_result_exists(&cx, root));

    let generation = engine.generation();
    engine
        .compute(&mut cx, root, AVAILABLE)
        .expect("cached compute succeeds");

    assert_eq!(engine.generation(), generation);
}

#[test]
fn mutation_invalidates_cache() {
    let (mut cx, root, _child) = build_tree();
    let mut engine = stub_engine();

    engine
        .compute(&mut cx, root, AVAILABLE)
        .expect("initial compute succeeds");
    let generation = engine.generation();

    cx.get_mut(root)
        .expect("root exists")
        .attach(LayoutStyle::flex_row().fixed_size(80.0, 24.0));
    engine
        .compute(&mut cx, root, AVAILABLE)
        .expect("mutated compute succeeds");

    assert_ne!(engine.generation(), generation);
}

#[test]
fn mutation_via_get_mut_invalidates_cache() {
    let (mut cx, root, child) = build_tree();
    let mut engine = stub_engine();

    engine
        .compute(&mut cx, root, AVAILABLE)
        .expect("initial compute succeeds");
    let generation = engine.generation();

    cx.get_mut(child)
        .expect("child exists")
        .attach(LayoutStyle::block().fixed_size(24.0, 12.0));
    cx.mark_mutated();
    engine
        .compute(&mut cx, root, AVAILABLE)
        .expect("marked get_mut compute succeeds");

    assert_ne!(engine.generation(), generation);
}

#[test]
fn compute_after_add_child_invalidates_cache() {
    let (mut cx, root, _child) = build_tree();
    let mut engine = stub_engine();

    engine
        .compute(&mut cx, root, AVAILABLE)
        .expect("initial compute succeeds");
    let generation = engine.generation();

    let added = cx.add_child(
        root,
        container().layout(LayoutStyle::block().fixed_size(16.0, 10.0)),
    );
    engine
        .compute(&mut cx, root, AVAILABLE)
        .expect("structural compute succeeds");

    assert_ne!(engine.generation(), generation);
    assert!(layout_result_exists(&cx, added));
}

#[test]
fn cached_compute_does_not_bump_layout_generation() {
    let (mut cx, root, _child) = build_tree();
    let mut engine = stub_engine();

    engine
        .compute(&mut cx, root, AVAILABLE)
        .expect("initial compute succeeds");
    let generation = engine.generation();

    engine
        .compute(&mut cx, root, AVAILABLE)
        .expect("first cached compute succeeds");
    engine
        .compute(&mut cx, root, AVAILABLE)
        .expect("second cached compute succeeds");

    assert_eq!(engine.generation(), generation);
}

#[test]
fn cached_compute_allocates_zero() {
    let (mut cx, root, _child) = build_tree();
    let mut engine = stub_engine();

    engine
        .compute(&mut cx, root, AVAILABLE)
        .expect("initial compute succeeds");

    let info = allocation_counter::measure(|| {
        engine
            .compute(&mut cx, root, AVAILABLE)
            .expect("cached compute succeeds");
    });

    assert_eq!(info.count_total, 0, "{info:?}");
}

#[test]
fn available_size_change_invalidates_cache() {
    let mut cx = Cx::new_root();
    let root = cx.add(container().layout(LayoutStyle::block()));
    let mut engine = stub_engine();

    engine
        .compute(&mut cx, root, Size::fixed(120.0, 40.0))
        .expect("initial compute succeeds");
    let generation = engine.generation();
    assert_eq!(layout_rect(&cx, root).width.to_bits(), 120.0_f32.to_bits());

    engine
        .compute(&mut cx, root, Size::fixed(240.0, 40.0))
        .expect("resized compute succeeds");

    assert_ne!(engine.generation(), generation);
    assert_eq!(layout_rect(&cx, root).width.to_bits(), 240.0_f32.to_bits());
}

#[test]
fn computing_different_root_in_same_cx_invalidates_cache() {
    let mut cx = Cx::new_root();
    let first = cx.add(container().layout(LayoutStyle::block().fixed_size(12.0, 10.0)));
    let second = cx.add(container().layout(LayoutStyle::block().fixed_size(24.0, 10.0)));
    let mut engine = stub_engine();

    engine
        .compute(&mut cx, first, AVAILABLE)
        .expect("first root compute succeeds");
    let generation = engine.generation();

    engine
        .compute(&mut cx, second, AVAILABLE)
        .expect("second root compute succeeds");

    assert_ne!(engine.generation(), generation);
    assert!(layout_result_exists(&cx, second));
    assert_eq!(layout_rect(&cx, second).width.to_bits(), 24.0_f32.to_bits());
}

#[test]
fn reusing_engine_with_fresh_cx_invalidates_cache() {
    let mut engine = stub_engine();
    let (mut first_cx, first_root, _first_child) = build_tree();

    engine
        .compute(&mut first_cx, first_root, AVAILABLE)
        .expect("first context compute succeeds");
    let generation = engine.generation();

    let (mut second_cx, second_root, _second_child) = build_tree();
    engine
        .compute(&mut second_cx, second_root, AVAILABLE)
        .expect("second context compute succeeds");

    assert_ne!(engine.generation(), generation);
    assert!(layout_result_exists(&second_cx, second_root));
}
