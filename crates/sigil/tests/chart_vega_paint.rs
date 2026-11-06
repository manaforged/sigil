#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "pixel coordinates are small and positive"
)]

use sigil::geometry::Rect;
use sigil::kernel::Role;
use sigil::prelude::*;
use sigil_kittest::{Harness, Query};

type Picture = image::RgbaImage;

fn paint(json: &'static str) -> (Harness, Picture) {
    let mut ui = Harness::new(move || {
        column()
            .bg(Token::Surface)
            .child(markdown(signal(format!("```vega-lite\n{json}\n```"))))
    })
    .pixels(800, 700);
    ui.settle();
    let picture = ui.image().expect("render").expect("gpu");
    (ui, picture)
}

fn label(ui: &mut Harness, text: &str) -> Rect {
    ui.find_all(&Query::role(Role::Label))
        .into_iter()
        .filter(|node| node.value() == Some(text))
        .filter_map(|node| node.bounds())
        .max_by(|a, b| a.origin.y.total_cmp(&b.origin.y))
        .unwrap_or_else(|| panic!("no label {text:?}: {}", ui.text()))
}

fn top_label(ui: &mut Harness, text: &str) -> Rect {
    ui.find_all(&Query::role(Role::Label))
        .into_iter()
        .filter(|node| node.value() == Some(text))
        .filter_map(|node| node.bounds())
        .min_by(|a, b| a.origin.y.total_cmp(&b.origin.y))
        .unwrap_or_else(|| panic!("no label {text:?}: {}", ui.text()))
}

fn inked(picture: &Picture, x: u32, y: u32) -> bool {
    let [r, g, b, _] = picture.get_pixel(x, y).0;
    let (high, low) = (r.max(g).max(b), r.min(g).min(b));
    high - low > 40
}

fn inked_box(picture: &Picture, top: u32, bottom: u32) -> Option<(u32, u32, u32, u32)> {
    let mut found: Option<(u32, u32, u32, u32)> = None;
    for y in top..bottom {
        for x in 0..picture.width() {
            if inked(picture, x, y) {
                let (l, t, r, b) = found.unwrap_or((x, y, x, y));
                found = Some((l.min(x), t.min(y), r.max(x), b.max(y)));
            }
        }
    }
    found
}

fn middle_y(rect: Rect) -> u32 {
    (rect.origin.y + rect.height / 2.0) as u32
}

fn ink_in_column(picture: &Picture, x: u32, top: u32, bottom: u32) -> bool {
    (top..bottom).any(|y| inked(picture, x, y))
}

#[test]
#[ignore = "needs a wgpu adapter"]
fn a_negative_bar_chart_draws_a_zero_rule() {
    let (mut ui, picture) = paint(
        r#"{"data":{"values":[{"a":"A","b":-28},{"a":"B","b":55},{"a":"C","b":-33},{"a":"D","b":91}]},"mark":"bar","encoding":{"x":{"field":"a","type":"nominal"},"y":{"field":"b","type":"quantitative"}}}"#,
    );
    let (zero, a, d) = (label(&mut ui, "0"), label(&mut ui, "A"), label(&mut ui, "D"));
    let y = middle_y(zero);
    let dark = (a.origin.x as u32..(d.origin.x + d.width) as u32)
        .filter(|x| {
            (y.saturating_sub(1)..=y + 1).any(|row| {
                let [r, g, b, _] = picture.get_pixel(*x, row).0;
                r.max(g).max(b) < 200
            })
        })
        .count();
    let span = (d.origin.x + d.width - a.origin.x) as usize;
    let b = label(&mut ui, "B");
    let gap = ((a.origin.x + b.origin.x + b.width) / 2.0) as u32;
    let probe = (y.saturating_sub(2)..y + 3)
        .map(|row| picture.get_pixel(gap, row).0)
        .collect::<Vec<_>>();
    assert!(dark * 100 >= span * 98, "zero row {y} covers {dark} of {span} px, gap {probe:?}");
}

#[test]
#[ignore = "needs a wgpu adapter"]
fn a_null_breaks_the_line() {
    let (mut ui, picture) = paint(
        r#"{"data":{"values":[{"x":1,"y":10},{"x":2,"y":30},{"x":3,"y":null},{"x":4,"y":15},{"x":5,"y":30},{"x":6,"y":40},{"x":7,"y":20}]},"mark":"line","encoding":{"x":{"field":"x","type":"quantitative"},"y":{"field":"y","type":"quantitative"}}}"#,
    );
    let (two, four, top) = (label(&mut ui, "2"), label(&mut ui, "4"), label(&mut ui, "40"));
    let gap = ((two.origin.x + four.origin.x + four.width) / 2.0) as u32;
    let (from, to) = (top.origin.y as u32, two.origin.y as u32 - 4);
    assert!(!ink_in_column(&picture, gap, from, to), "the line bridges x=3");
    let five = label(&mut ui, "5");
    let joined = (five.origin.x + five.width / 2.0) as u32;
    assert!(ink_in_column(&picture, joined, from, to), "no line at x=5");
}

#[test]
#[ignore = "needs a wgpu adapter"]
fn a_null_breaks_the_area() {
    let (mut ui, picture) = paint(
        r#"{"data":{"values":[{"x":1,"y":10},{"x":2,"y":30},{"x":3,"y":null},{"x":4,"y":15},{"x":5,"y":30},{"x":6,"y":40},{"x":7,"y":20}]},"mark":"area","encoding":{"x":{"field":"x","type":"quantitative"},"y":{"field":"y","type":"quantitative"}}}"#,
    );
    let (two, four, top) = (label(&mut ui, "2"), label(&mut ui, "4"), label(&mut ui, "40"));
    let gap = ((two.origin.x + four.origin.x + four.width) / 2.0) as u32;
    assert!(
        !ink_in_column(&picture, gap, top.origin.y as u32, two.origin.y as u32 - 4),
        "the area bridges x=3"
    );
}

fn one_point(mark: &'static str) -> (u32, u32, u32, u32, Picture) {
    let json: &'static str = match mark {
        "point" => r#"{"data":{"values":[{"x":5,"y":5}]},"mark":"point","encoding":{"x":{"field":"x","type":"quantitative"},"y":{"field":"y","type":"quantitative"}}}"#,
        _ => r#"{"data":{"values":[{"x":5,"y":5}]},"mark":"square","encoding":{"x":{"field":"x","type":"quantitative"},"y":{"field":"y","type":"quantitative"}}}"#,
    };
    let (mut ui, picture) = paint(json);
    let (top, zero) = (top_label(&mut ui, "5.0"), top_label(&mut ui, "0.0"));
    let found = inked_box(&picture, top.origin.y as u32, zero.origin.y as u32);
    let (l, t, r, b) = found.expect("no point drawn");
    (l, t, r, b, picture)
}

#[test]
#[ignore = "needs a wgpu adapter"]
fn a_point_mark_is_an_open_circle() {
    let (l, t, r, b, picture) = one_point("point");
    assert!(r - l >= 4 && b - t >= 4, "point box {l},{t} {r},{b}");
    assert!(!inked(&picture, (l + r) / 2, (t + b) / 2), "the point is filled");
}

#[test]
#[ignore = "needs a wgpu adapter"]
fn a_square_mark_fills_its_corners() {
    let (l, t, r, b, picture) = one_point("square");
    assert!(inked(&picture, (l + r) / 2, (t + b) / 2), "the square is hollow");
    assert!(inked(&picture, l + 1, t + 1), "the square has round corners");
}

#[test]
#[ignore = "needs a wgpu adapter"]
fn a_color_range_paints_the_slices() {
    let (_, picture) = paint(
        r##"{"data":{"values":[{"c":1,"v":4},{"c":2,"v":6}]},"mark":"arc","encoding":{"theta":{"field":"v","type":"quantitative"},"color":{"field":"c","type":"nominal","scale":{"range":["#ff0000","#00ff00"]}}}}"##,
    );
    let (mut red, mut green) = (0, 0);
    for pixel in picture.pixels() {
        let [r, g, b, _] = pixel.0;
        red += usize::from(r > 230 && g < 30 && b < 30);
        green += usize::from(g > 230 && r < 30 && b < 30);
    }
    assert!(red > 100 && green > 100, "red {red}, green {green}");
}
