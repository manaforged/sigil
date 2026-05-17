#![allow(missing_docs)]
#![allow(clippy::unwrap_used)]

use ahash as _;
use allocation_counter as _;
use criterion as _;
use rayon as _;
use sigil_hal as _;
use sigil_reactive as _;
use sigil_style as _;
use smallvec as _;
use taffy as _;
mod stub;
use stub::stub_engine;

use sigil_kernel::prelude::*;
use sigil_layout::prelude::*;

const EPSILON: f32 = 0.5;

#[test]
fn flex_row_grow_child_fills_remaining_space() {
    let mut cx = Cx::new_root();
    let root = cx.add(container().child(text("A")).child(text("B")));
    let children = cx
        .get(root)
        .expect("root exists")
        .children
        .iter()
        .copied()
        .collect::<Vec<_>>();

    cx.get_mut(root).expect("root exists").attach(
        LayoutStyle::flex_row()
            .fixed_size(100.0, 20.0)
            .align_items(AlignItems::FlexStart),
    );
    let second = children.get(1).copied().expect("second child id exists");
    cx.get_mut(second)
        .expect("second child exists")
        .attach(LayoutStyle::default().grow(1.0));

    let mut engine = stub_engine();
    engine
        .compute(&mut cx, root, Size::fixed(100.0, 20.0))
        .expect("compute succeeds");

    let second = cx
        .get(second)
        .and_then(|node| node.get_extension::<LayoutResult>())
        .expect("second child result");
    assert_close(second.rect().width, 92.0);
}

#[test]
fn flex_column_gap_offsets_children() {
    let mut cx = Cx::new_root();
    let root = cx.add(container().child(text("A")).child(text("B")));
    let children = cx
        .get(root)
        .expect("root exists")
        .children
        .iter()
        .copied()
        .collect::<Vec<_>>();

    cx.get_mut(root).expect("root exists").attach(
        LayoutStyle::flex_column()
            .gap_px(4.0)
            .align_items(AlignItems::FlexStart),
    );

    let mut engine = stub_engine();
    engine
        .compute(&mut cx, root, Size::max_content())
        .expect("compute succeeds");

    let second = cx
        .get(children.get(1).copied().expect("second child id exists"))
        .and_then(|node| node.get_extension::<LayoutResult>())
        .expect("second child result");
    assert_close(second.rect().origin.y, 20.0);
}

#[test]
fn padding_moves_content_origin() {
    let mut cx = Cx::new_root();
    let root = cx.add(container().child(text("A")));
    let child = cx
        .get(root)
        .and_then(|node| node.children.first())
        .copied()
        .expect("child exists");

    cx.get_mut(root).expect("root exists").attach(
        LayoutStyle::flex_row()
            .padding_px(6.0)
            .align_items(AlignItems::FlexStart),
    );

    let mut engine = stub_engine();
    engine
        .compute(&mut cx, root, Size::max_content())
        .expect("compute succeeds");

    let result = cx
        .get(child)
        .and_then(|node| node.get_extension::<LayoutResult>())
        .expect("child result");
    assert_close(result.rect().origin.x, 6.0);
    assert_close(result.rect().origin.y, 6.0);
}

#[test]
fn block_auto_width_fills_available_space() {
    let mut cx = Cx::new_root();
    let root = cx.add(container().child(container().child(text("block"))));
    let child = cx
        .get(root)
        .and_then(|node| node.children.first())
        .copied()
        .expect("child exists");

    cx.get_mut(root)
        .expect("root exists")
        .attach(LayoutStyle::block().fixed_size(200.0, 50.0));
    cx.get_mut(child)
        .expect("child exists")
        .attach(LayoutStyle::block().height_px(20.0));

    let mut engine = stub_engine();
    engine
        .compute(&mut cx, root, Size::fixed(200.0, 50.0))
        .expect("compute succeeds");

    let result = cx
        .get(child)
        .and_then(|node| node.get_extension::<LayoutResult>())
        .expect("child result");
    assert_close(result.rect().width, 200.0);
}

#[test]
fn grid_2x2_places_items_in_fixed_tracks() {
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .child(text("one"))
            .child(text("two"))
            .child(text("three"))
            .child(text("four")),
    );
    let children = cx
        .get(root)
        .expect("root exists")
        .children
        .iter()
        .copied()
        .collect::<Vec<_>>();

    cx.get_mut(root).expect("root exists").attach(
        LayoutStyle::grid()
            .fixed_size(200.0, 100.0)
            .with_taffy(|style| {
                style.grid_template_columns = vec![length(80.0), length(120.0)];
                style.grid_template_rows = vec![length(40.0), length(60.0)];
            }),
    );

    let mut engine = stub_engine();
    engine
        .compute(&mut cx, root, Size::fixed(200.0, 100.0))
        .expect("compute succeeds");

    let first = result_for(&cx, children.first().copied().expect("first child exists"));
    let second = result_for(&cx, children.get(1).copied().expect("second child exists"));
    let third = result_for(&cx, children.get(2).copied().expect("third child exists"));

    assert_close(first.rect().origin.x, 0.0);
    assert_close(first.rect().origin.y, 0.0);
    assert_close(second.rect().origin.x, 80.0);
    assert_close(second.rect().origin.y, 0.0);
    assert_close(third.rect().origin.x, 0.0);
    assert_close(third.rect().origin.y, 40.0);
}

#[test]
fn nested_absolute_uses_positioned_parent_origin() {
    let mut cx = Cx::new_root();
    let root = cx.add(container().child(container().child(text("abs"))));
    let parent = cx
        .get(root)
        .and_then(|node| node.children.first())
        .copied()
        .expect("parent exists");
    let child = cx
        .get(parent)
        .and_then(|node| node.children.first())
        .copied()
        .expect("absolute child exists");

    cx.get_mut(root)
        .expect("root exists")
        .attach(LayoutStyle::flex_row().fixed_size(200.0, 100.0));
    cx.get_mut(parent).expect("parent exists").attach(
        LayoutStyle::flex_row()
            .fixed_size(120.0, 80.0)
            .padding_px(5.0),
    );
    cx.get_mut(child).expect("child exists").attach(
        LayoutStyle::default()
            .fixed_size(30.0, 20.0)
            .with_taffy(|style| {
                style.position = Position::Absolute;
                style.inset.left = LengthPercentageAuto::length(10.0);
                style.inset.top = LengthPercentageAuto::length(12.0);
            }),
    );

    let mut engine = stub_engine();
    engine
        .compute(&mut cx, root, Size::fixed(200.0, 100.0))
        .expect("compute succeeds");

    let result = result_for(&cx, child);
    assert_close(result.rect().origin.x, 10.0);
    assert_close(result.rect().origin.y, 12.0);
    assert_close(result.rect().width, 30.0);
    assert_close(result.rect().height, 20.0);
}

/// Fragment children should appear as direct siblings of the fragment's parent,
/// meaning they receive the same gap spacing as real children of that parent.
#[test]
fn fragment_children_treated_as_siblings_of_parent() {
    let mut cx = Cx::new_root();
    // root (flex-column, gap=4) contains: text("A"), fragment { text("B"), text("C") }
    // After flattening the fragment, layout sees three siblings: A, B, C.
    // B should start at ~20px (height of A + 4px gap), C at ~44px (20+4+16+4).
    let root = cx.add(
        container()
            .child(text("A"))
            .child(fragment().child(text("B")).child(text("C"))),
    );

    cx.get_mut(root).expect("root exists").attach(
        LayoutStyle::flex_column()
            .gap_px(4.0)
            .align_items(AlignItems::FlexStart),
    );

    let mut engine = stub_engine();
    engine
        .compute(&mut cx, root, Size::max_content())
        .expect("compute succeeds");

    // Collect the three text nodes from the tree.
    let root_children = cx.get(root).expect("root").children.clone();
    let text_a = root_children.first().copied().expect("text A");
    let frag = root_children.get(1).copied().expect("fragment node");
    let frag_children = cx.get(frag).expect("fragment").children.clone();
    let text_b = frag_children.first().copied().expect("text B");
    let text_c = frag_children.get(1).copied().expect("text C");

    let result_a = result_for(&cx, text_a);
    let result_b = result_for(&cx, text_b);
    let result_c = result_for(&cx, text_c);

    // A is at the top.
    assert_close(result_a.rect().origin.y, 0.0);
    // B is one text-height + gap below A.
    let b_expected_y = result_a.rect().height + 4.0;
    assert_close(result_b.rect().origin.y, b_expected_y);
    // C is one text-height + gap below B.
    let c_expected_y = b_expected_y + result_b.rect().height + 4.0;
    assert_close(result_c.rect().origin.y, c_expected_y);
}

fn result_for(cx: &Cx, id: NodeId) -> &LayoutResult {
    cx.get(id)
        .and_then(|node| node.get_extension::<LayoutResult>())
        .expect("layout result exists")
}

fn assert_close(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() <= EPSILON,
        "expected {actual} to be within {EPSILON} of {expected}",
    );
}
