#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]

use std::cell::Cell;
use std::ops::Range;
use std::rc::Rc;

use sigil::kernel::{Cx, Role};
use sigil::prelude::*;
use sigil::reactive::Runtime;
use sigil::widgets::{ChartBuilder, chart};
use sigil_kittest::{Harness, Query};

fn line_build_time(points: usize) -> std::time::Duration {
    Runtime::scope("chart-build", || {
        let mut cx = Cx::new_root();
        let values: Vec<f32> = (0..points).map(|at| (at as f32 * 0.01).sin()).collect();
        let started = std::time::Instant::now();
        let _ = cx.add(chart(values).line().width(400.0).height(200.0));
        started.elapsed()
    })
}

#[test]
fn a_line_of_100k_points_costs_about_the_same_as_1k() {
    let small = line_build_time(1_000);
    let large = line_build_time(100_000);
    assert!(
        large < small * 10,
        "100k points took {large:?}, 1k points {small:?}"
    );
}

#[test]
fn the_wheel_zooms_a_bound_x_range() {
    type RangeSlot = Rc<Cell<Option<Signal<Range<f32>>>>>;
    let slot: RangeSlot = Rc::new(Cell::new(None));
    let export = Rc::clone(&slot);
    let mut ui = Harness::new(move || {
        let range = signal(0.0_f32..100.0);
        export.set(Some(range));
        chart((0..100).map(|at| at as f32).collect::<Vec<_>>())
            .line()
            .x_range(range)
            .height(160.0)
    })
    .size(60, 20);
    ui.settle();
    let plot = ui.get_by_role(Role::Figure, "Chart");
    ui.hover(&plot).scroll(&plot, -3.0);
    ui.settle();
    let Range { start, end } = slot.get().expect("range signal").peek();
    assert!(end - start < 100.0, "the range stayed {start}..{end}");
}

#[test]
fn a_crosshair_reads_out_the_value_under_the_pointer() {
    let mut ui = Harness::new(|| {
        chart(vec![3.0, 7.0, 5.0])
            .line()
            .crosshair()
            .height(160.0)
            .width(300.0)
    })
    .size(60, 20);
    ui.settle();
    let plot = ui.get_by_role(Role::Figure, "Chart");
    ui.hover(&plot);
    ui.settle();
    let readout = ui
        .find_all(&Query::role(Role::Label))
        .into_iter()
        .filter_map(|node| node.value().map(str::to_owned))
        .any(|value| ["3", "7", "5"].contains(&value.trim()));
    assert!(readout, "no value readout after hovering the plot");
}

#[test]
#[ignore = "needs a wgpu adapter"]
fn rising_and_falling_candles_draw_in_two_colours() {
    let mut ui = Harness::new(|| {
        column().bg(Token::Surface).child(
            ChartBuilder::candles(vec![(1.0, 4.0, 0.5, 3.5), (3.5, 4.0, 1.0, 1.5)])
                .height(200.0)
                .width(300.0),
        )
    })
    .pixels(320, 220);
    ui.settle();
    let image = ui.image().unwrap().expect("gpu");
    let (mut green, mut red) = (0, 0);
    for pixel in image.pixels() {
        let [r, g, _, _] = pixel.0;
        if g > r.saturating_add(60) {
            green += 1;
        }
        if r > g.saturating_add(60) {
            red += 1;
        }
    }
    assert!(green > 0 && red > 0, "green {green}, red {red}");
}
