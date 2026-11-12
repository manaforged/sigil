#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use crate::widget_support;

use sigil::kernel::Cx;
use sigil::prelude::*;
use sigil::widgets::{Orientation, splitter};
use sigil_kittest::FakeMeasure;
use widget_support::compute;

#[test]
fn horizontal_splitter_lays_out_panes_with_initial_sizes() {
    let mut cx = Cx::new_root();
    let root = cx.add(
        splitter(Orientation::Horizontal)
            .child(container().label("a"), 200.0)
            .child(container().label("b"), 200.0)
            .child(container().label("c"), 200.0),
    );

    let layout = compute(
        &mut cx,
        root,
        Constraint::fixed(612.0, 200.0),
        Box::new(FakeMeasure::default()),
    );

    let pane_0 = cx.nth_child(root, 0).expect("pane 0");
    let pane_1 = cx.nth_child(root, 2).expect("pane 1");
    let pane_2 = cx.nth_child(root, 4).expect("pane 2");

    let w0 = layout.result(pane_0).expect("pane 0 layout").rect().width;
    let w1 = layout.result(pane_1).expect("pane 1 layout").rect().width;
    let w2 = layout.result(pane_2).expect("pane 2 layout").rect().width;

    assert!(
        (w0 - 200.0).abs() < 0.5,
        "pane 0 expected ~200 px wide, got {w0}",
    );
    assert!(
        (w1 - 200.0).abs() < 0.5,
        "pane 1 expected ~200 px wide, got {w1}",
    );
    assert!(
        (w2 - 200.0).abs() < 0.5,
        "pane 2 expected ~200 px wide, got {w2}",
    );
}

#[test]
fn vertical_splitter_lays_out_panes_with_initial_sizes() {
    let mut cx = Cx::new_root();
    let root = cx.add(
        splitter(Orientation::Vertical)
            .child(container().label("top"), 100.0)
            .child(container().label("bottom"), 100.0),
    );

    let layout = compute(
        &mut cx,
        root,
        Constraint::fixed(300.0, 206.0),
        Box::new(FakeMeasure::default()),
    );

    let pane_top = cx.nth_child(root, 0).expect("top pane");
    let pane_bottom = cx.nth_child(root, 2).expect("bottom pane");

    let h_top = layout.result(pane_top).expect("top layout").rect().height;
    let h_bottom = layout
        .result(pane_bottom)
        .expect("bottom layout")
        .rect()
        .height;

    assert!(
        (h_top - 100.0).abs() < 0.5,
        "top expected ~100 px tall, got {h_top}",
    );
    assert!(
        (h_bottom - 100.0).abs() < 0.5,
        "bottom expected ~100 px tall, got {h_bottom}",
    );
}

#[test]
fn nested_splitter_renders_all_leaves() {
    let inner = splitter(Orientation::Horizontal)
        .child(container().label("bottom-left"), 150.0)
        .child(container().label("bottom-right"), 150.0);

    let mut cx = Cx::new_root();
    let root = cx.add(
        splitter(Orientation::Vertical)
            .child(container().label("top"), 100.0)
            .child(inner, 100.0),
    );

    let _layout = compute(
        &mut cx,
        root,
        Constraint::fixed(306.0, 206.0),
        Box::new(FakeMeasure::default()),
    );

    let bottom_pane = cx.nth_child(root, 2).expect("bottom pane");
    let inner_root = cx.nth_child(bottom_pane, 0).expect("inner splitter root");

    let left = cx.nth_child(inner_root, 0).expect("bottom-left");
    let right = cx.nth_child(inner_root, 2).expect("bottom-right");

    assert!(cx.get(left).is_some(), "bottom-left leaf must exist");
    assert!(cx.get(right).is_some(), "bottom-right leaf must exist");
}

#[test]
fn pane_builder_carries_initial_min_max_into_layout() {
    let mut cx = Cx::new_root();
    let root = cx.add(
        splitter(Orientation::Horizontal)
            .pane(container().label("a"))
            .width(150.0)
            .min_width(50.0)
            .max_width(300.0)
            .pane(container().label("b"))
            .width(250.0)
            .min_width(100.0)
            .max_width(400.0),
    );

    let layout = compute(
        &mut cx,
        root,
        Constraint::fixed(406.0, 200.0),
        Box::new(FakeMeasure::default()),
    );

    let pane_a = cx.nth_child(root, 0).expect("pane a");
    let pane_b = cx.nth_child(root, 2).expect("pane b");

    let wa = layout.result(pane_a).expect("pane a layout").rect().width;
    let wb = layout.result(pane_b).expect("pane b layout").rect().width;

    assert!(
        (wa - 150.0).abs() < 0.5,
        "pane a expected ~150 px wide, got {wa}",
    );
    assert!(
        (wb - 250.0).abs() < 0.5,
        "pane b expected ~250 px wide, got {wb}",
    );
}

#[test]
fn grow_pane_takes_leftover_width() {
    sigil::reactive::Owner::scope("phase-2-4::splitter-grow", || {
        let mut cx = Cx::new_root();
        let root = cx.add(
            splitter(Orientation::Horizontal)
                .pane(container().label("side"))
                .width(120.0)
                .pane(container().label("main"))
                .grow(1.0)
                .commit()
                .width(406.0)
                .height(200.0),
        );
        let layout = compute(
            &mut cx,
            root,
            Constraint::fixed(406.0, 200.0),
            Box::new(FakeMeasure::default()),
        );
        let side = cx.nth_child(root, 0).expect("side pane");
        let main = cx.nth_child(root, 2).expect("main pane");
        let side_w = layout.result(side).expect("side").rect().width;
        let main_w = layout.result(main).expect("main").rect().width;
        assert!(
            (side_w - 120.0).abs() < 0.5,
            "fixed pane stays 120, got {side_w}"
        );
        assert!(
            (main_w - 280.0).abs() < 0.5,
            "grow pane takes leftover (406 - 120 - 6 handle), got {main_w}"
        );
    });
}

#[test]
fn sizes_bind_carries_initial_sizes() {
    sigil::reactive::Owner::scope("phase-2-4::splitter-sizes-bind", || {
        let bound = signal(Vec::<f32>::new());
        let mut cx = Cx::new_root();
        let _root = cx.add(
            splitter(Orientation::Horizontal)
                .child(container().label("a"), 100.0)
                .child(container().label("b"), 150.0)
                .child(container().label("c"), 200.0)
                .sizes_bind(bound),
        );

        let snapshot = bound.peek();
        assert_eq!(
            snapshot,
            vec![100.0, 150.0, 200.0],
            "sizes_bind must reflect initial sizes after build",
        );
    });
}

#[test]
fn dragging_the_handle_resizes_the_panes() {
    let pane = |name: &'static str| {
        container()
            .role(Role::Image)
            .label(name)
            .layout(LayoutStyle::flex_column().fill())
    };
    let mut ui = sigil_kittest::Harness::new(
        splitter(Orientation::Horizontal)
            .child(pane("left"), 200.0)
            .child(pane("right"), 200.0),
    )
    .pixels(600, 200);
    let handle = ui.get(sigil_kittest::Query::role(Role::Splitter));
    let from = handle.center().expect("handle bounds");
    ui.drag(&handle, &Point::new(from.x + 50.0, from.y))
        .settle();
    let left = ui.get((Role::Image, "left")).bounds().expect("left bounds");
    let right = ui
        .get((Role::Image, "right"))
        .bounds()
        .expect("right bounds");
    assert!(
        (left.width - 250.0).abs() < 1.0,
        "left pane is {} px wide",
        left.width
    );
    assert!(
        (right.width - 150.0).abs() < 1.0,
        "right pane is {} px wide",
        right.width
    );
}

fn shared(width: u32) -> sigil_kittest::Harness {
    let pane = |name: &'static str| {
        container()
            .role(Role::Image)
            .label(name)
            .layout(LayoutStyle::flex_column().fill())
    };
    sigil_kittest::Harness::new(
        splitter(Orientation::Horizontal)
            .proportional()
            .pane(pane("left"))
            .width(1.0)
            .min_width(100.0)
            .pane(pane("right"))
            .width(3.0)
            .min_width(100.0)
            .commit(),
    )
    .pixels(width, 200)
}

fn widths(ui: &mut sigil_kittest::Harness) -> (f32, f32) {
    let width = |harness: &mut sigil_kittest::Harness, name| {
        harness
            .get((Role::Image, name))
            .bounds()
            .expect("pane bounds")
            .width
    };
    (width(ui, "left"), width(ui, "right"))
}

#[test]
fn a_proportional_splitter_keeps_each_panes_share_at_any_width() {
    for (width, left, right) in [(406, 100.0, 300.0), (806, 200.0, 600.0)] {
        let mut ui = shared(width);
        ui.settle();
        let (got_left, got_right) = widths(&mut ui);
        assert!(
            (got_left - left).abs() < 1.0 && (got_right - right).abs() < 1.0,
            "at {width} px the panes are {got_left} and {got_right}",
        );
    }
}

#[test]
fn a_proportional_drag_stops_at_the_other_panes_minimum() {
    let mut ui = shared(806);
    ui.settle();
    let handle = ui.get(sigil_kittest::Query::role(Role::Splitter));
    let from = handle.center().expect("handle bounds");
    ui.drag(&handle, &Point::new(from.x + 700.0, from.y))
        .settle();
    let (left, right) = widths(&mut ui);
    assert!((right - 100.0).abs() < 1.0, "right pane is {right} px wide");
    assert!((left - 700.0).abs() < 1.0, "left pane is {left} px wide");
}

#[test]
fn settle_bind_reports_sizes_when_the_drag_ends() {
    sigil::reactive::Owner::scope("settle", settle_after_drag);
}

fn settle_after_drag() {
    let settled = signal(Vec::<f32>::new());
    let mut ui = sigil_kittest::Harness::new(
        splitter(Orientation::Horizontal)
            .child(container(), 200.0)
            .child(container(), 200.0)
            .settle_bind(settled),
    )
    .pixels(600, 200);
    assert!(settled.peek().is_empty(), "nothing settles before a drag");
    let handle = ui.get(sigil_kittest::Query::role(Role::Splitter));
    let from = handle.center().expect("handle bounds");
    ui.drag(&handle, &Point::new(from.x + 50.0, from.y))
        .settle();
    assert_eq!(settled.peek(), vec![250.0, 150.0]);
}
