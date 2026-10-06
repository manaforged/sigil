#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]

use sigil::layout::Position;
use sigil::prelude::*;
use sigil::reactive::Runtime;
use sigil::style::Transform2D;
use sigil_kittest::Harness;

#[test]
fn accessibility_bounds_follow_a_scaled_and_moved_parent() {
    Runtime::scope("a11y-transform", || {
        let mut ui = Harness::new(|| {
            container()
                .layout(LayoutStyle::block().width(400.0).height(300.0))
                .transform(
                    Transform2D::IDENTITY
                        .origin(0.0, 0.0)
                        .translate(100.0, 50.0)
                        .scale(2.0, 2.0),
                )
                .child(
                    container().role(Role::Image).label("shape").layout(
                        LayoutStyle::block()
                            .position(Position::Absolute)
                            .inset_left(10.0)
                            .inset_top(10.0)
                            .width(40.0)
                            .height(20.0),
                    ),
                )
        })
        .pixels(600, 400);
        ui.settle();
        let nodes = ui.accessibility_tree().nodes;
        let shape = nodes
            .iter()
            .map(|(_, node)| node)
            .find(|node| node.label() == Some("shape"))
            .and_then(accesskit::Node::bounds)
            .expect("shape bounds");
        assert_eq!(
            (shape.x0, shape.y0, shape.x1, shape.y1),
            (120.0, 70.0, 200.0, 110.0)
        );
    });
}
