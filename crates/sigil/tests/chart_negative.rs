#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used)]

use sigil::prelude::*;
use sigil::reactive::Runtime;
use sigil_kittest::{Harness, Query};

#[test]
fn a_negative_bar_hangs_below_the_zero_line() {
    Runtime::scope("chart-negative", || {
        let mut ui = Harness::new(|| column().child(chart(vec![-40.0, 60.0]).height(240.0)))
            .pixels(400, 300);
        ui.settle();
        let below = ui.get(&Query::text("-40")).bounds().unwrap();
        let above = ui.get(&Query::text("60")).bounds().unwrap();
        assert!(below.height > 10.0, "the -40 bar has no height: {below:?}");
        assert!(
            below.origin.y >= above.origin.y + above.height - 1.0,
            "the -40 bar {below:?} is not below the zero line under {above:?}"
        );
        assert!(
            ui.query(&Query::text("-50")).is_some(),
            "the y axis has no -50 tick"
        );
    });
}
