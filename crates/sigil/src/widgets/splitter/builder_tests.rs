#![allow(clippy::float_cmp)]

use crate::kernel::{Cx, NodeBuilder, container};
use crate::reactive::Owner;

use super::{Orientation, splitter};
use crate::geometry::Size;
use crate::layout::{FlexDirection, LayoutEngine, LayoutStyle, Length};
use crate::widgets::Widget;

#[test]
fn a_width_on_a_pane_sets_the_pane_and_not_the_splitter() {
    let pane = splitter(Orientation::Horizontal)
        .pane(container())
        .width(180.0);

    assert_eq!(pane.entry.initial_size, 180.0);
    assert_eq!(pane.splitter.layout_overrides.width_hint(), None);
}

#[test]
fn a_width_in_columns_on_a_pane_sets_its_initial_size() {
    let width = crate::layout::cols(30.0);
    let pane = splitter(Orientation::Horizontal)
        .pane(container())
        .width(width);

    assert_eq!(Some(pane.entry.initial_size), width.px());
    assert!(pane.entry.initial_size > 0.0);
}

#[test]
fn a_width_on_the_splitter_reaches_its_root() {
    Owner::scope("splitter-width", || {
        let mut cx = Cx::new_root();
        let root = cx.add(
            splitter(Orientation::Horizontal)
                .pane(container())
                .commit()
                .width(180.0),
        );
        let style = cx
            .get(root)
            .expect("root")
            .get_extension::<LayoutStyle>()
            .expect("layout");
        assert_eq!(style.data().size.width, Length::Px(180.0));
    });
}

#[test]
fn map_node_keeps_the_splitter_row() {
    Owner::scope("splitter-map-node", || {
        let mut cx = Cx::new_root();
        let root = cx.add(
            splitter(Orientation::Horizontal)
                .child(container(), 100.0)
                .child(container(), 100.0)
                .map_node(|node| node),
        );
        let style = cx
            .get(root)
            .expect("root")
            .get_extension::<LayoutStyle>()
            .expect("layout");
        assert_eq!(style.data().flex_direction, FlexDirection::Row);
    });
}

#[test]
fn fixed_pane_and_handle_keep_their_width_with_filling_content() {
    Owner::scope("splitter-fixed-pane", || {
        let mut cx = Cx::new_root();
        let root = cx.add(
            splitter(Orientation::Horizontal)
                .pane(container().layout(LayoutStyle::flex_column().fill()))
                .width(224.0)
                .min_width(180.0)
                .max_width(500.0)
                .pane(container().layout(LayoutStyle::flex_column().fill().min_size(0.0, 0.0)))
                .grow(1.0)
                .min_width(320.0)
                .commit()
                .layout(LayoutStyle::flex_row().fill()),
        );
        let mut layout =
            LayoutEngine::with_text_measure(Box::new(crate::layout::FakeMeasure::fixed(0.0, 0.0)));
        layout
            .compute(&mut cx, root, Size::fixed(1280.0, 900.0))
            .expect("layout");
        let first = cx.nth_child(root, 0).expect("sidebar");
        let handle = cx.nth_child(root, 1).expect("handle");
        let second = cx.nth_child(root, 2).expect("content");
        let sidebar = layout.result(first).expect("sidebar bounds").rect();
        let divider = layout.result(handle).expect("divider bounds").rect();
        let content = layout.result(second).expect("content bounds").rect();
        assert_eq!(sidebar.width, 224.0);
        assert_eq!(divider.width, super::HANDLE_PX);
        assert_eq!(divider.origin.x, sidebar.width);
        assert_eq!(content.origin.x, divider.origin.x + divider.width);
        assert_eq!(content.origin.x + content.width, 1280.0);
    });
}

#[test]
fn splitter_and_pane_builders_convert_to_nodes() {
    Owner::scope("splitter-into", || {
        let two = || {
            splitter(Orientation::Horizontal)
                .child(container(), 120.0)
                .child(container(), 200.0)
        };
        let pane: NodeBuilder = splitter(Orientation::Horizontal)
            .pane(container())
            .width(120.0)
            .pane(container())
            .grow(1.0)
            .into();
        let into: NodeBuilder = two().into();
        let mut cx = Cx::new_root();
        for id in [cx.add(two()), cx.add(into), cx.add(pane)] {
            assert!(cx.get(id).is_some());
        }
    });
}
