#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used)]

#[path = "layout_stub/mod.rs"]
mod stub;
use sigil_kittest::FixedMeasure;
use stub::stub_engine;

use sigil::geometry::Size;
use sigil::kernel::*;
use sigil::layout::{
    Dimension as StyleDimension, Display, LayoutDrive, LengthPercentageAuto, Position, StyleData, *,
};

const EPSILON: f32 = 0.5;

#[test]
fn flex_row_grow_child_fills_remaining_space() {
    let mut cx = Cx::new_root();
    let root = cx.add(container().child(text("A")).child(text("B")));
    let children = cx.get(root).expect("root exists").children().to_vec();

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
    let children = cx.get(root).expect("root exists").children().to_vec();

    cx.get_mut(root).expect("root exists").attach(
        LayoutStyle::flex_column()
            .gap(4.0)
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
        .and_then(|node| node.children().first().copied())
        .expect("child exists");

    cx.get_mut(root).expect("root exists").attach(
        LayoutStyle::flex_row()
            .padding(6.0)
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
        .and_then(|node| node.children().first().copied())
        .expect("child exists");

    cx.get_mut(root)
        .expect("root exists")
        .attach(LayoutStyle::block().fixed_size(200.0, 50.0));
    cx.get_mut(child)
        .expect("child exists")
        .attach(LayoutStyle::block().height(20.0));

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
fn nested_absolute_uses_positioned_parent_origin() {
    let mut cx = Cx::new_root();
    let root = cx.add(container().child(container().child(text("abs"))));
    let parent = cx
        .get(root)
        .and_then(|node| node.children().first().copied())
        .expect("parent exists");
    let child = cx
        .get(parent)
        .and_then(|node| node.children().first().copied())
        .expect("absolute child exists");

    cx.get_mut(root)
        .expect("root exists")
        .attach(LayoutStyle::flex_row().fixed_size(200.0, 100.0));
    cx.get_mut(parent)
        .expect("parent exists")
        .attach(LayoutStyle::flex_row().fixed_size(120.0, 80.0).padding(5.0));
    cx.get_mut(child).expect("child exists").attach(
        LayoutStyle::default()
            .fixed_size(30.0, 20.0)
            .with_layout(|style| {
                style.position = Position::Absolute;
                style.inset.left = LengthPercentageAuto::Px(10.0);
                style.inset.top = LengthPercentageAuto::Px(12.0);
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

#[test]
fn absolute_child_uses_inset_offset() {
    let mut drive = LayoutDrive::new(Box::new(FixedMeasure::default()));
    let mut parent_style = StyleData {
        display: Display::Flex,
        ..StyleData::default()
    };
    parent_style.size.width = StyleDimension::Px(200.0);
    parent_style.size.height = StyleDimension::Px(100.0);

    let mut child_style = StyleData {
        position: Position::Absolute,
        ..StyleData::default()
    };
    child_style.size.width = StyleDimension::Px(30.0);
    child_style.size.height = StyleDimension::Px(20.0);
    child_style.inset.left = LengthPercentageAuto::Px(20.0);
    child_style.inset.top = LengthPercentageAuto::Px(50.0);

    let root = drive.insert(parent_style);
    let child = drive.insert(child_style);
    drive.attach(root, child).expect("attach succeeds");
    drive
        .measure(root, sigil::layout::Size::new(200.0, 100.0))
        .expect("measure succeeds");

    let result = drive.computed_box(child).expect("child result");
    assert_close(result.x, 20.0);
    assert_close(result.y, 50.0);
}

#[test]
fn fragment_children_treated_as_siblings_of_parent() {
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .child(text("A"))
            .child(fragment().child(text("B")).child(text("C"))),
    );

    cx.get_mut(root).expect("root exists").attach(
        LayoutStyle::flex_column()
            .gap(4.0)
            .align_items(AlignItems::FlexStart),
    );

    let mut engine = stub_engine();
    engine
        .compute(&mut cx, root, Size::max_content())
        .expect("compute succeeds");

    let root_children: Vec<_> = cx.get(root).expect("root").children().to_vec();
    let text_a = root_children.first().copied().expect("text A");
    let frag = root_children.get(1).copied().expect("fragment node");
    let frag_children: Vec<_> = cx.get(frag).expect("fragment").children().to_vec();
    let text_b = frag_children.first().copied().expect("text B");
    let text_c = frag_children.get(1).copied().expect("text C");

    let result_a = result_for(&cx, text_a);
    let result_b = result_for(&cx, text_b);
    let result_c = result_for(&cx, text_c);

    assert_close(result_a.rect().origin.y, 0.0);
    let b_expected_y = result_a.rect().height + 4.0;
    assert_close(result_b.rect().origin.y, b_expected_y);
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
