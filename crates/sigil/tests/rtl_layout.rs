#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::expect_used)]

use sigil::kernel::Role;
use sigil::layout::TrackSize;
use sigil::prelude::*;
use sigil::style::{Direction, Theme};
use sigil_kittest::Harness;

fn x_of(direction: Direction, view: fn() -> NodeBuilder, name: &str) -> f32 {
    let mut ui = Harness::new(move || {
        sigil::app::set_theme(Theme::light().with_direction(direction));
        view()
    })
    .pixels(600, 200);
    ui.settle();
    ui.get_by_role(Role::Button, name)
        .bounds()
        .expect("bounds")
        .origin
        .x
}

fn block_view() -> NodeBuilder {
    container()
        .layout(LayoutStyle::block().width(600.0))
        .child(button("Narrow").layout(LayoutStyle::block().width(100.0)))
}

fn grid_view() -> NodeBuilder {
    container()
        .layout(LayoutStyle::grid(vec![TrackSize::Px(200.0), TrackSize::Px(200.0)]).width(600.0))
        .child(button("First"))
        .child(button("Second"))
}

#[test]
fn a_narrow_block_child_sits_at_the_right_in_rtl() {
    let ltr = x_of(Direction::Ltr, block_view, "Narrow");
    let rtl = x_of(Direction::Rtl, block_view, "Narrow");
    assert!(rtl > ltr + 400.0, "ltr {ltr}, rtl {rtl}");
}

#[test]
fn grid_columns_run_right_to_left_in_rtl() {
    let first = x_of(Direction::Rtl, grid_view, "First");
    let second = x_of(Direction::Rtl, grid_view, "Second");
    assert!(first > second, "first {first}, second {second}");
}
