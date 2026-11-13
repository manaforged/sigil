#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::HashSet;

use sigil::icons::Icon;
use sigil::kernel::Role;
use sigil::prelude::*;
use sigil::style::{BgGradient, Color, ColorValue, GradientAxis, GradientStopKey};
use sigil_kittest::{Harness, Query};

#[test]
fn widget_builders_collect_into_one_child_list() {
    let grid = sigil_kittest::Harness::new(|| {
        let items: Vec<NodeBuilder> = vec![text("alpha").into(), button("beta").into()];
        items.into_iter().fold(column(), NodeBuilder::child)
    })
    .size(30, 6)
    .text();
    assert!(grid.contains("alpha") && grid.contains("beta"), "{grid}");
}

#[test]
fn signals_compare_by_identity() {
    sigil::reactive::Owner::scope("signal-identity", || {
        let first = signal(1_u8);
        let twin = signal(1_u8);
        let set: HashSet<Signal<u8>> = [first, first, twin].into_iter().collect();
        assert_eq!(first, first);
        assert_ne!(first, twin);
        assert_eq!(set.len(), 2);
    });
}

#[test]
fn an_icon_button_is_named_by_its_icon() {
    let mut ui = Harness::new(|| button(Icon::SkipForward).activate(|| {})).size(40, 6);
    ui.settle();
    assert!(
        ui.query(&Query::role(Role::Button).name("skip-forward"))
            .is_some(),
        "no button named skip-forward"
    );
}

#[test]
fn equal_owned_gradient_stops_share_one_copy() {
    let stops = || {
        vec![
            GradientStopKey::new(
                0,
                ColorValue::Color(Color {
                    r: 255,
                    g: 0,
                    b: 0,
                    a: 255,
                }),
            ),
            GradientStopKey::new(
                1000,
                ColorValue::Color(Color {
                    r: 0,
                    g: 0,
                    b: 255,
                    a: 255,
                }),
            ),
        ]
    };
    let first = BgGradient::owned(GradientAxis::LeftToRight, stops());
    let second = BgGradient::owned(GradientAxis::LeftToRight, stops());
    assert_eq!(first.stops, stops().as_slice());
    assert!(std::ptr::eq(first.stops, second.stops));
}

#[test]
fn a_converted_builder_is_the_widget_node_itself() {
    sigil::reactive::Owner::scope("node-from", || {
        let mut cx = sigil::kernel::Cx::new_root();
        let converted: NodeBuilder = text("hello").into();
        let root = cx.add(column().child(converted.label("greeting")));
        let child = cx.nth_child(root, 0).expect("child");
        let node = cx.get(child).expect("node");
        assert!(matches!(node.kind, sigil::kernel::NodeType::Text { .. }));
        assert!(matches!(
            &node.a11y.name,
            sigil::kernel::AccessibleName::Static(name) if name == "greeting"
        ));
    });
}

#[test]
fn children_closures_and_kittest_targets_work_ergonomically() {
    let borrowed = String::from("borrowed");
    assert_eq!(TextContent::from(&borrowed).resolve(), "borrowed");
    let _sel: Selection = SelectionMode::Multi.into();
    let hits = std::rc::Rc::new(std::cell::Cell::new(0_u32));
    let count = std::rc::Rc::clone(&hits);
    let mut ui = Harness::new(move || {
        let count = std::rc::Rc::clone(&count);
        column()
            .children(["alpha", "beta"].map(text))
            .child(text(move || "live-str"))
            .child(button("Run").activate(move || count.set(count.get() + 1)))
    })
    .size(40, 12);
    ui.expect_text("alpha")
        .expect_text("beta")
        .expect_text("live-str");
    ui.click(&String::from("Run"));
    ui.click(&(Role::Button, "Run"));
    assert!(ui.query(&Query::from(Role::Button)).is_some());
    assert_eq!(hits.get(), 2);
}
