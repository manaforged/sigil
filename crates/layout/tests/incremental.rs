#![allow(missing_docs)]
#![allow(clippy::unwrap_used)]

use ahash as _;
use allocation_counter as _;
use criterion as _;
use sigil_hal as _;
use sigil_style as _;
use smallvec as _;
use taffy as _;

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

fn build_tree() -> (Cx, NodeId, NodeId) {
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .layout(LayoutStyle::flex_row().fixed_size(120.0, 40.0))
            .child(container().layout(LayoutStyle::block().fixed_size(12.0, 10.0))),
    );
    let child = cx
        .get(root)
        .and_then(|node| node.children.first())
        .copied()
        .expect("child exists");
    (cx, root, child)
}

fn layout_result_exists(cx: &Cx, id: NodeId) -> bool {
    cx.get(id)
        .and_then(|node| node.get_extension::<LayoutResult>())
        .is_some()
}

fn layout_rect(cx: &Cx, id: NodeId) -> sigil_hal::Rect {
    cx.get(id)
        .and_then(|node| node.get_extension::<LayoutResult>())
        .expect("layout result exists")
        .rect()
}

struct ContentWidthMeasure;

impl TextMeasure for ContentWidthMeasure {
    fn measure(
        &self,
        content: &TextContent,
        _space: taffy::prelude::Size<taffy::prelude::AvailableSpace>,
    ) -> taffy::prelude::Size<f32> {
        let len = match content {
            TextContent::Static(value) => value.len(),
            TextContent::Reactive(value) => value.peek().len(),
        };
        let clamped = u16::try_from(len).unwrap_or(u16::MAX);
        taffy::prelude::Size {
            width: f32::from(clamped) * 10.0,
            height: 10.0,
        }
    }
}
