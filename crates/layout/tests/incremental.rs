#![allow(unused_crate_dependencies)]
#![allow(missing_docs)]
#![allow(clippy::unwrap_used)]

use ahash as _;
use allocation_counter as _;
use criterion as _;
use rayon as _;
use sigil_hal as _;
use sigil_style as _;
use smallvec as _;

mod stub;

use sigil_hal::Size;
use sigil_kernel::prelude::*;
use sigil_layout::prelude::*;
use sigil_reactive::{Runtime, signal};
use sigil_style::{Dimension, NodeStyleExt, Px, StyleAtom};
use stub::stub_engine;

const AVAILABLE: Size = Size::fixed(120.0, 40.0);
static NARROW: &[StyleAtom] = &[StyleAtom::Width(Dimension::Px(Px(32)))];
static WIDE: &[StyleAtom] = &[StyleAtom::Width(Dimension::Px(Px(96)))];

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

    cx.mark_mutated();
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

#[test]
fn reactive_text_change_invalidates_cache() {
    Runtime::scope("layout_reactive_text_cache", || {
        let value = signal(String::from("tiny"));
        let mut cx = Cx::new_root();
        let root = cx.add(text(value));
        let mut engine = LayoutEngine::with_text_measure(Box::new(ContentWidthMeasure));

        engine
            .compute(&mut cx, root, Size::max_content())
            .expect("initial text compute succeeds");
        let generation = engine.generation();
        let initial_width = layout_rect(&cx, root).width;

        value.set(String::from("much wider"));
        engine
            .compute(&mut cx, root, Size::max_content())
            .expect("changed text compute succeeds");

        assert_ne!(engine.generation(), generation);
        assert!(layout_rect(&cx, root).width > initial_width);
    });
}

#[test]
fn reactive_layout_style_change_invalidates_cache() {
    Runtime::scope("layout_reactive_style_cache", || {
        let wide = signal(false);
        let mut cx = Cx::new_root();
        let root =
            cx.add(container().style_reactive(move || if wide.get() { WIDE } else { NARROW }));
        let mut engine = stub_engine();

        engine
            .compute(&mut cx, root, AVAILABLE)
            .expect("initial style compute succeeds");
        let generation = engine.generation();
        assert_eq!(layout_rect(&cx, root).width.to_bits(), 32.0_f32.to_bits());

        wide.set(true);
        engine
            .compute(&mut cx, root, AVAILABLE)
            .expect("changed style compute succeeds");

        assert_ne!(engine.generation(), generation);
        assert_eq!(layout_rect(&cx, root).width.to_bits(), 96.0_f32.to_bits());
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

fn build_tree() -> (Cx, NodeId, NodeId) {
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .layout(LayoutStyle::flex_row().fixed_size(120.0, 40.0))
            .child(container().layout(LayoutStyle::block().fixed_size(12.0, 10.0))),
    );
    let child = cx
        .get(root)
        .and_then(|node| node.children().first().copied())
        .expect("child exists");
    (cx, root, child)
}

fn build_many_leaf_row(count: usize) -> (Cx, NodeId, Vec<NodeId>) {
    let mut cx = Cx::new_root();
    let root = cx.add(container().layout(LayoutStyle::flex_row().fixed_size(200.0, 40.0)));
    let children = (0..count)
        .map(|_| {
            cx.add_child(
                root,
                container().layout(LayoutStyle::block().fixed_size(12.0, 10.0)),
            )
        })
        .collect();
    (cx, root, children)
}

fn ordered_fixed(width: f32, height: f32, order: i32) -> LayoutStyle {
    LayoutStyle::block()
        .fixed_size(width, height)
        .with_layout(|style| style.order = order)
}

fn layout_result_exists(cx: &Cx, id: NodeId) -> bool {
    cx.get(id)
        .and_then(|node| node.get_extension::<LayoutResult>())
        .is_some()
}

fn layout_result(cx: &Cx, id: NodeId) -> LayoutResult {
    *cx.get(id)
        .and_then(|node| node.get_extension::<LayoutResult>())
        .expect("layout result exists")
}

fn layout_rect(cx: &Cx, id: NodeId) -> sigil_hal::Rect {
    cx.get(id)
        .and_then(|node| node.get_extension::<LayoutResult>())
        .expect("layout result exists")
        .rect()
}

struct ContentWidthMeasure;

impl TextMeasure for ContentWidthMeasure {
    fn measure(&self, content: &TextContent, _space: sigil_hal::Size) -> sigil_hal::LogicalSize {
        let len = match content {
            TextContent::Static(value) => value.len(),
            TextContent::Reactive(value) => value.peek().len(),
        };
        let clamped = u16::try_from(len).unwrap_or(u16::MAX);
        sigil_hal::LogicalSize::new(f32::from(clamped) * 10.0, 10.0)
    }
}
