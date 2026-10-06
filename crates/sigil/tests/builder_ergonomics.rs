#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::HashSet;

use sigil::icons::Icon;
use sigil::kernel::Role;
use sigil::prelude::*;
use sigil::style::{BgGradient, ColorAtom, GradientAxis, GradientStopKey, Rgba8};
use sigil_kittest::{Harness, Query};

#[test]
fn widget_builders_collect_into_one_child_list() {
    let grid = sigil::terminal::present(
        || {
            let items: Vec<NodeBuilder> = vec![text("alpha").into(), button("beta").into()];
            items.into_iter().fold(column(), NodeBuilder::child)
        },
        30,
        6,
        &[],
    )
    .expect("layout");
    assert!(grid.contains("alpha") && grid.contains("beta"), "{grid}");
}

#[test]
fn signals_compare_by_identity() {
    sigil::reactive::Runtime::scope("signal-identity", || {
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
    let mut ui = Harness::new(|| icon_button(Icon::SkipForward, || {})).size(40, 6);
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
                ColorAtom::Rgba8(Rgba8 {
                    r: 255,
                    g: 0,
                    b: 0,
                    a: 255,
                }),
            ),
            GradientStopKey::new(
                1000,
                ColorAtom::Rgba8(Rgba8 {
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
