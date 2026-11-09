#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    reason = "pixel coordinates are small and positive"
)]
#![allow(
    clippy::indexing_slicing,
    clippy::panic,
    reason = "a test fails loudly on a missing value"
)]

use std::time::{Duration, Instant};

use sigil::kernel::Role;
use sigil::prelude::*;
use sigil_kittest::{Harness, Query};

type Picture = image::RgbaImage;

const WAIT: Duration = Duration::from_secs(20);
const GAP_RATIO: f32 = 0.33;
const GAP_TOLERANCE: f32 = 0.03;
const SLICE_RATIO: f32 = 1.5;
const SLICE_TOLERANCE: f32 = 0.05;
const RULED_SHARE: f32 = 0.98;

struct Chart {
    picture: Picture,
    paper: [u8; 3],
}

fn paint(json: &'static str) -> Chart {
    let mut ui = Harness::new(move || {
        column()
            .bg(Token::Surface)
            .child(markdown(signal(format!("```vega-lite\n{json}\n```"))))
    })
    .pixels(800, 700);
    let start = Instant::now();
    ui.settle();
    while ui.query(&Query::text("Rendering chart")).is_some() {
        assert!(start.elapsed() < WAIT, "still rendering after {WAIT:?}");
        std::thread::sleep(Duration::from_millis(5));
        ui.settle();
    }
    let rect = ui
        .find_all(&Query::role(Role::Image))
        .into_iter()
        .find_map(|node| node.bounds())
        .unwrap_or_else(|| panic!("no chart image: {}", ui.text()));
    let surface = ui.host().theme().color(Token::Surface);
    let full = ui.image().expect("render").expect("gpu");
    let picture = image::imageops::crop_imm(
        &full,
        rect.origin.x.round() as u32,
        rect.origin.y.round() as u32,
        rect.width.round() as u32,
        rect.height.round() as u32,
    )
    .to_image();
    Chart {
        picture,
        paper: [surface.r, surface.g, surface.b],
    }
}

fn inked(picture: &Picture, x: u32, y: u32) -> bool {
    let [red, green, blue, _] = picture.get_pixel(x, y).0;
    let (high, low) = (red.max(green).max(blue), red.min(green).min(blue));
    high - low > 40
}

fn ruled(chart: &Chart, x: u32, y: u32) -> bool {
    let pixel = chart.picture.get_pixel(x, y).0;
    pixel
        .iter()
        .zip(chart.paper)
        .any(|(got, paper)| got.abs_diff(paper) > 8)
}

fn inked_columns(picture: &Picture) -> Vec<(u32, u32)> {
    let mut runs: Vec<(u32, u32)> = Vec::new();
    for x in 0..picture.width() {
        if !(0..picture.height()).any(|y| inked(picture, x, y)) {
            continue;
        }
        match runs.last_mut() {
            Some((_, end)) if *end + 1 == x => *end = x,
            _ => runs.push((x, x)),
        }
    }
    runs
}

fn inked_rows(picture: &Picture, x: u32) -> (u32, u32) {
    let rows: Vec<u32> = (0..picture.height())
        .filter(|y| inked(picture, x, *y))
        .collect();
    (rows[0], rows[rows.len() - 1])
}

fn inked_box(picture: &Picture) -> (u32, u32, u32, u32) {
    let mut found: Option<(u32, u32, u32, u32)> = None;
    for (x, y, _) in picture.enumerate_pixels() {
        if inked(picture, x, y) {
            let (l, t, r, b) = found.unwrap_or((x, y, x, y));
            found = Some((l.min(x), t.min(y), r.max(x), b.max(y)));
        }
    }
    found.expect("no mark drawn")
}

fn assert_gap(json: &'static str, mark: &str) {
    let chart = paint(json);
    let runs = inked_columns(&chart.picture);
    assert_eq!(
        runs.len(),
        2,
        "the {mark} has {} inked spans: {runs:?}",
        runs.len()
    );
    let (first, last) = (runs[0], runs[1]);
    let ratio = (last.0 - first.1) as f32 / (last.1 - first.0) as f32;
    assert!(
        (ratio - GAP_RATIO).abs() <= GAP_TOLERANCE,
        "the {mark} gap at x=3 spans {ratio:.3} of the width, the reference {GAP_RATIO}"
    );
}

#[test]
#[ignore = "needs a wgpu adapter"]
fn a_negative_bar_chart_draws_a_zero_rule() {
    let chart = paint(
        r#"{"data":{"values":[{"a":"A","b":-28},{"a":"B","b":55},{"a":"C","b":-33},{"a":"D","b":91}]},"mark":"bar","encoding":{"x":{"field":"a","type":"nominal"},"y":{"field":"b","type":"quantitative"}}}"#,
    );
    let bars = inked_columns(&chart.picture);
    assert_eq!(bars.len(), 4, "bars {bars:?}");
    let ends: Vec<(u32, u32)> = bars
        .iter()
        .map(|(l, r)| inked_rows(&chart.picture, u32::midpoint(*l, *r)))
        .collect();
    let zero = ends[0].0;
    assert_eq!(
        ends[2].0, zero,
        "the negative bars start at different rows: {ends:?}"
    );
    let meets = |bottom: u32| bottom + 1 == zero || bottom == zero;
    assert!(
        meets(ends[1].1) && meets(ends[3].1),
        "the positive bars do not end on the zero row {zero}: {ends:?}"
    );
    let (left, right) = (bars[0].0, bars[3].1);
    let covered = (left..=right).filter(|x| ruled(&chart, *x, zero)).count();
    let span = (right - left + 1) as usize;
    assert!(
        covered as f32 >= span as f32 * RULED_SHARE,
        "zero row {zero} covers {covered} of {span} px"
    );
}

#[test]
#[ignore = "needs a wgpu adapter"]
fn a_null_breaks_the_line() {
    assert_gap(
        r#"{"data":{"values":[{"x":1,"y":10},{"x":2,"y":30},{"x":3,"y":null},{"x":4,"y":15},{"x":5,"y":30},{"x":6,"y":40},{"x":7,"y":20}]},"mark":"line","encoding":{"x":{"field":"x","type":"quantitative"},"y":{"field":"y","type":"quantitative"}}}"#,
        "line",
    );
}

#[test]
#[ignore = "needs a wgpu adapter"]
fn a_null_breaks_the_area() {
    assert_gap(
        r#"{"data":{"values":[{"x":1,"y":10},{"x":2,"y":30},{"x":3,"y":null},{"x":4,"y":15},{"x":5,"y":30},{"x":6,"y":40},{"x":7,"y":20}]},"mark":"area","encoding":{"x":{"field":"x","type":"quantitative"},"y":{"field":"y","type":"quantitative"}}}"#,
        "area",
    );
}

#[test]
#[ignore = "needs a wgpu adapter"]
fn a_point_mark_is_an_open_circle() {
    let chart = paint(
        r#"{"data":{"values":[{"x":5,"y":5}]},"mark":"point","encoding":{"x":{"field":"x","type":"quantitative"},"y":{"field":"y","type":"quantitative"}}}"#,
    );
    let (l, t, r, b) = inked_box(&chart.picture);
    assert!(r - l >= 6 && b - t >= 6, "point box {l},{t} {r},{b}");
    let center = (u32::midpoint(l, r), u32::midpoint(t, b));
    assert!(
        !inked(&chart.picture, center.0, center.1),
        "the point is filled"
    );
}

#[test]
#[ignore = "needs a wgpu adapter"]
fn a_square_mark_fills_its_corners() {
    let chart = paint(
        r#"{"data":{"values":[{"x":5,"y":5}]},"mark":"square","encoding":{"x":{"field":"x","type":"quantitative"},"y":{"field":"y","type":"quantitative"}}}"#,
    );
    let (l, t, r, b) = inked_box(&chart.picture);
    assert!(r - l >= 4 && b - t >= 4, "square box {l},{t} {r},{b}");
    let center = (u32::midpoint(l, r), u32::midpoint(t, b));
    assert!(
        inked(&chart.picture, center.0, center.1),
        "the square is hollow"
    );
    assert!(
        inked(&chart.picture, l + 1, t + 1),
        "the square has round corners"
    );
}

#[test]
#[ignore = "needs a wgpu adapter"]
fn a_color_range_paints_the_slices() {
    let chart = paint(
        r##"{"data":{"values":[{"c":1,"v":4},{"c":2,"v":6}]},"mark":"arc","encoding":{"theta":{"field":"v","type":"quantitative"},"color":{"field":"c","type":"nominal","scale":{"range":["#ff0000","#00ff00"]}}}}"##,
    );
    let (mut red, mut green) = (0_u32, 0_u32);
    for pixel in chart.picture.pixels() {
        let [r, g, b, _] = pixel.0;
        red += u32::from(r > 230 && g < 30 && b < 30);
        green += u32::from(g > 230 && r < 30 && b < 30);
    }
    assert!(red > 100 && green > 100, "red {red}, green {green}");
    let ratio = green as f32 / red as f32;
    assert!(
        (ratio - SLICE_RATIO).abs() <= SLICE_TOLERANCE,
        "green to red is {ratio:.3}, the reference {SLICE_RATIO}"
    );
}
