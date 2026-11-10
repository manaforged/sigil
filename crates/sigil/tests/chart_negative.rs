#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used)]

use sigil::geometry::Rect;
use sigil::kernel::Role;
use sigil::prelude::*;
use sigil::reactive::Owner;
use sigil_kittest::{Harness, Query};

fn bar(ui: &mut Harness, caption: &str) -> Rect {
    ui.find_all(&Query::role(Role::GenericContainer).name(caption))
        .into_iter()
        .find_map(|node| node.bounds())
        .unwrap()
}

fn tick(ui: &mut Harness, text: &str) -> bool {
    ui.find_all(&Query::role(Role::Label))
        .iter()
        .any(|node| node.value() == Some(text))
}

#[test]
fn a_negative_bar_hangs_below_the_zero_line() {
    Owner::scope("chart-negative", || {
        let mut ui = Harness::new(|| column().child(chart(vec![-40.0, 60.0]).height(240.0)))
            .pixels(400, 300);
        ui.settle();
        let below = bar(&mut ui, "-40");
        let above = bar(&mut ui, "60");
        assert!(below.height > 10.0, "the -40 bar has no height: {below:?}");
        assert!(
            below.origin.y >= above.origin.y + above.height - 1.0,
            "the -40 bar {below:?} is not below the zero line under {above:?}"
        );
        assert!(
            tick(&mut ui, "-40") && tick(&mut ui, "0") && !tick(&mut ui, "-50"),
            "the y axis does not tick -40 and 0"
        );
    });
}
