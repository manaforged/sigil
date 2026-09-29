#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used)]

use std::hint::black_box;
use std::time::Duration;

use criterion::{Criterion, criterion_group, criterion_main};
use sigil::geometry::Size;
use sigil::kernel::{Cx, NodeBuilder, NodeId, container};
use sigil::layout::{LayoutBuilderExt, LayoutEngine, LayoutStyle};
use sigil_kittest::FixedMeasure;
use taffy::prelude::*;

const ROOT_WIDTH: f32 = 12_800.0;
const ROOT_HEIGHT: f32 = 100.0;
const CHILD_WIDTH: f32 = 12.0;
const CHILD_HEIGHT: f32 = 100.0;
const LEAF_WIDTH: f32 = 12.0;
const LEAF_HEIGHT: f32 = 1.0;
const CHILDREN: usize = 1_000;
const LEAVES_PER_CHILD: usize = 100;
const AVAILABLE: Size = Size::fixed(ROOT_WIDTH, ROOT_HEIGHT);

const TEXT_MEASURE: FixedMeasure = FixedMeasure {
    width: 0.0,
    height: 0.0,
};

const fn fixed_available(width: f32, height: f32) -> taffy::Size<taffy::AvailableSpace> {
    taffy::Size {
        width: taffy::AvailableSpace::Definite(width),
        height: taffy::AvailableSpace::Definite(height),
    }
}

fn taffy_fixed_style(
    display: taffy::Display,
    direction: taffy::FlexDirection,
    width: f32,
    height: f32,
) -> taffy::Style {
    taffy::Style {
        display,
        flex_direction: direction,
        size: taffy::Size {
            width: taffy::Dimension::from_length(width),
            height: taffy::Dimension::from_length(height),
        },
        ..Default::default()
    }
}

fn build_owned(cx: &mut Cx) -> (NodeId, NodeId) {
    let mut root = container().layout(LayoutStyle::flex_row().fixed_size(ROOT_WIDTH, ROOT_HEIGHT));
    for _ in 0..CHILDREN {
        root = root.child(column());
    }
    let root_id = cx.add(root);
    let first_col = cx.nth_child(root_id, 0).expect("root has a first column");
    let leaf = cx.nth_child(first_col, 0).expect("column has a first leaf");
    (root_id, leaf)
}

fn column() -> NodeBuilder {
    let mut child =
        container().layout(LayoutStyle::flex_column().fixed_size(CHILD_WIDTH, CHILD_HEIGHT));
    for _ in 0..LEAVES_PER_CHILD {
        child = child
            .child(container().layout(LayoutStyle::block().fixed_size(LEAF_WIDTH, LEAF_HEIGHT)));
    }
    child
}

fn bench_owned_incremental(c: &mut Criterion) {
    c.bench_function("incremental_one_leaf/owned", |b| {
        let mut cx = Cx::new_root();
        let (root, leaf) = build_owned(&mut cx);
        let mut engine = LayoutEngine::with_text_measure(Box::new(TEXT_MEASURE));
        engine
            .compute(&mut cx, root, AVAILABLE)
            .expect("bench harness invariant");
        cx.clear_dirty();

        let mut width = LEAF_WIDTH;
        b.iter(|| {
            width = if width >= LEAF_WIDTH + 10.0 {
                LEAF_WIDTH
            } else {
                width + 1.0
            };
            cx.get_mut(leaf)
                .expect("leaf is live")
                .attach(LayoutStyle::block().fixed_size(width, LEAF_HEIGHT));
            engine
                .compute(&mut cx, root, AVAILABLE)
                .expect("bench harness invariant");
            black_box(engine.result(root));
        });
    });
}

fn build_taffy(tree: &mut taffy::TaffyTree<()>) -> (taffy::NodeId, taffy::NodeId) {
    let leaf_style = taffy_fixed_style(
        taffy::Display::Block,
        taffy::FlexDirection::Row,
        LEAF_WIDTH,
        LEAF_HEIGHT,
    );
    let mut columns = Vec::with_capacity(CHILDREN);
    let mut first_leaf = None;
    for col_idx in 0..CHILDREN {
        let mut leaves = Vec::with_capacity(LEAVES_PER_CHILD);
        for leaf_idx in 0..LEAVES_PER_CHILD {
            let leaf = tree
                .new_leaf(leaf_style.clone())
                .expect("bench harness invariant");
            if col_idx == 0 && leaf_idx == 0 {
                first_leaf = Some(leaf);
            }
            leaves.push(leaf);
        }
        let column = tree
            .new_with_children(
                taffy_fixed_style(
                    taffy::Display::Flex,
                    taffy::FlexDirection::Column,
                    CHILD_WIDTH,
                    CHILD_HEIGHT,
                ),
                &leaves,
            )
            .expect("bench harness invariant");
        columns.push(column);
    }
    let root = tree
        .new_with_children(
            taffy_fixed_style(
                taffy::Display::Flex,
                taffy::FlexDirection::Row,
                ROOT_WIDTH,
                ROOT_HEIGHT,
            ),
            &columns,
        )
        .expect("bench harness invariant");
    (root, first_leaf.expect("at least one leaf"))
}

fn bench_taffy_incremental(c: &mut Criterion) {
    c.bench_function("incremental_one_leaf/taffy", |b| {
        let mut tree = taffy::TaffyTree::<()>::new();
        let (root, leaf) = build_taffy(&mut tree);
        tree.compute_layout(root, fixed_available(ROOT_WIDTH, ROOT_HEIGHT))
            .expect("bench harness invariant");

        let mut width = LEAF_WIDTH;
        b.iter(|| {
            width = if width >= LEAF_WIDTH + 10.0 {
                LEAF_WIDTH
            } else {
                width + 1.0
            };
            tree.set_style(
                leaf,
                taffy_fixed_style(
                    taffy::Display::Block,
                    taffy::FlexDirection::Row,
                    width,
                    LEAF_HEIGHT,
                ),
            )
            .expect("bench harness invariant");
            tree.compute_layout(root, fixed_available(ROOT_WIDTH, ROOT_HEIGHT))
                .expect("bench harness invariant");
            black_box(tree.layout(root).expect("bench harness invariant"));
        });
    });
}

criterion_group!(
    name = benches;
    config = Criterion::default()
        .sample_size(50)
        .warm_up_time(Duration::from_secs(2))
        .measurement_time(Duration::from_secs(5));
    targets = bench_owned_incremental, bench_taffy_incremental,
);
criterion_main!(benches);
