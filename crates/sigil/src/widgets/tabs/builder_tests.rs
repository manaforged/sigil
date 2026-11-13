#![allow(clippy::unwrap_used, clippy::panic, clippy::float_cmp)]

use super::*;
use crate::geometry::Constraint;
use crate::kernel::Cx;
use crate::kernel::{AccessibleName, NodeId, Role};
use crate::layout::LayoutEngine;
use crate::reactive::{Owner, signal};
use crate::style::Theme;
use crate::style::render_style_for;
use crate::widgets::Widget;

fn find(cx: &Cx, root: NodeId, role: Role, label: &str) -> NodeId {
    let mut found = None;
    cx.visit_preorder(root, |id, node| {
        if node.a11y.role == role
            && matches!(&node.a11y.name, AccessibleName::Static(name) if name == label)
        {
            found = Some(id);
        }
    });
    found.unwrap()
}

#[test]
fn chrome_keeps_tabs_centered_and_reserves_its_height() {
    for (chrome, height) in [
        (TabsBuilder::line as fn(_) -> _, 36.0),
        (TabsBuilder::pill, 84.0),
    ] {
        Owner::scope("tabs-chrome", || {
            let mut cx = Cx::new_root();
            let strip = chrome(tabs().labels(["A", "B"])).label("test");
            let root = cx.add(
                container()
                    .layout(LayoutStyle::flex_column().fill())
                    .child(strip)
                    .child(
                        container()
                            .label("body")
                            .layout(LayoutStyle::block().height(100.0)),
                    ),
            );
            let mut layout =
                LayoutEngine::new(Box::new(crate::layout::FakeMeasure::fixed(40.0, 16.0)));
            layout
                .compute(&mut cx, root, Constraint::fixed(800.0, 600.0))
                .unwrap();
            let list = layout
                .result(find(&cx, root, Role::TabList, "test"))
                .unwrap()
                .rect();
            let body = layout
                .result(find(&cx, root, Role::GenericContainer, "body"))
                .unwrap()
                .rect();
            assert_eq!(list.origin.x + list.width / 2.0, 400.0);
            assert_eq!(body.origin.y, height);
            assert!(list.origin.y + list.height <= body.origin.y);
        });
    }
}

#[test]
fn selection_moves_the_painted_highlight_between_existing_tabs() {
    Owner::scope("tabs-selection", || {
        let active = signal(0);
        let theme = Theme::light();
        let mut cx = Cx::new_root();
        let root = cx.add(tabs().active(active).labels(["A", "B"]));
        cx.flush_reactive_children();
        let first = find(&cx, root, Role::Tab, "A");
        let second = find(&cx, root, Role::Tab, "B");
        let color = |id| render_style_for(cx.get(id).unwrap(), &theme, None).background_color;
        let selected = Some(theme.color(Token::SurfacePressed));
        let resting = Some(theme.color(Token::Surface));
        assert_ne!(selected, resting);
        assert_eq!((color(first), color(second)), (selected, resting));
        active.set(1);
        assert_eq!((color(first), color(second)), (resting, selected));
        active.set(0);
        assert_eq!((color(first), color(second)), (selected, resting));
    });
}
