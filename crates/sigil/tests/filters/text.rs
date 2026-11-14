use sigil::geometry::{Point, Rect};
use sigil::kernel::container;
use sigil::layout::LayoutStyle;
use sigil::prelude::*;
use sigil::scene::{DrawRect, SceneBuilder, Shadow, shape_glyph_run};
use sigil::style::TextStyle;
use sigil::widgets::canvas;

use super::{SCALE, SIDE, frames_of, rgba};

const INK: Color = rgba(255, 255, 255, 255);
const RED: [f32; 4] = [1.0, 0.0, 0.0, 1.0];
const BLUE: [f32; 4] = [0.0, 0.0, 1.0, 1.0];

fn label(scene: &mut SceneBuilder, rect: Rect, shadows: &[Shadow]) {
    scene.draw_rect(DrawRect::new(rect, rgba(0, 0, 0, 255)));
    let style = TextStyle::default().font_size(40.0).font_weight(800);
    let at = Rect::new(
        Point::new(rect.origin.x + 6.0, rect.origin.y + 4.0),
        50.0,
        50.0,
    );
    let run = shape_glyph_run(at, "HI".to_owned(), INK, style, 10_000);
    for shadow in shadows.iter().rev() {
        scene.draw_text_shadow(&run, *shadow);
    }
    scene.draw_glyph_run(run);
}

fn shadowed(shadows: Vec<Shadow>) -> Vec<(&'static str, image::RgbaImage)> {
    frames_of(move || {
        let shadows = shadows.clone();
        let paint = canvas(
            move |scene: &mut SceneBuilder, rect: Rect, _theme: &Theme| {
                label(scene, rect, &shadows);
            },
        )
        .width(SIDE as f32)
        .height(SIDE as f32);
        container()
            .layout(LayoutStyle::block().fixed_size(SIDE as f32, SIDE as f32))
            .child(paint)
    })
}

fn ink(frame: &image::RgbaImage, x: i64, y: i64) -> bool {
    let inside = x >= 0 && y >= 0 && x < i64::from(frame.width()) && y < i64::from(frame.height());
    inside && frame.get_pixel(x as u32, y as u32).0[1] >= 250
}

fn untouched(frame: &image::RgbaImage, x: i64, y: i64) -> bool {
    let inside = x >= 0 && y >= 0 && x < i64::from(frame.width()) && y < i64::from(frame.height());
    !inside || clear(frame, x as u32, y as u32)
}

fn clear(frame: &image::RgbaImage, x: u32, y: u32) -> bool {
    frame.get_pixel(x, y).0[..3].iter().all(|c| *c <= 2)
}

struct Hits {
    checked: usize,
    wrong: Vec<(u32, u32, [u8; 4])>,
}

fn under_offset(
    plain: &image::RgbaImage,
    shaded: &image::RgbaImage,
    offset: (i64, i64),
    expect: impl Fn([u8; 4]) -> bool,
) -> Hits {
    let mut hits = Hits {
        checked: 0,
        wrong: Vec::new(),
    };
    for (x, y, pixel) in shaded.enumerate_pixels() {
        let (sx, sy) = (i64::from(x) - offset.0, i64::from(y) - offset.1);
        if clear(plain, x, y) && ink(plain, sx, sy) {
            hits.checked += 1;
            if !expect(pixel.0) {
                hits.wrong.push((x, y, pixel.0));
            }
        }
    }
    hits
}

fn device(offset: f32) -> i64 {
    (offset * SCALE).round() as i64
}

#[test]
fn a_text_shadow_shows_its_color_below_and_right_of_the_glyphs() {
    let plain = shadowed(Vec::new());
    let shadow = Shadow::new([2.0, 2.0], 0.0, 0.0, RED);
    for ((name, base), (_, shaded)) in plain.iter().zip(shadowed(vec![shadow])) {
        let hits = under_offset(base, &shaded, (device(2.0), device(2.0)), |p| {
            p[0] >= 250 && p[1] <= 5
        });
        println!(
            "{name}: {} shadow pixels checked, {} wrong",
            hits.checked,
            hits.wrong.len()
        );
        assert!(
            hits.checked > 50,
            "{name}: the probe found shadow pixels to check"
        );
        assert!(
            hits.wrong.is_empty(),
            "{name}: shadow pixels not red: {:?}",
            &hits.wrong[..hits.wrong.len().min(5)]
        );
        let covered = base
            .enumerate_pixels()
            .filter(|(_, _, p)| p.0[1] >= 250)
            .all(|(x, y, p)| {
                shaded
                    .get_pixel(x, y)
                    .0
                    .iter()
                    .zip(p.0)
                    .all(|(a, b)| a.abs_diff(b) <= 2)
            });
        assert!(covered, "{name}: the shadow never paints over the text");
    }
}

#[test]
fn the_first_text_shadow_is_on_top_and_a_blurred_one_spreads() {
    let plain = shadowed(Vec::new());
    let first = Shadow::new([2.0, 2.0], 0.0, 0.0, RED);
    let second = Shadow::new([4.0, 4.0], 0.0, 0.0, BLUE);
    for ((name, base), (_, shaded)) in plain.iter().zip(shadowed(vec![first, second])) {
        let top = under_offset(base, &shaded, (device(2.0), device(2.0)), |p| {
            p[0] >= 250 && p[2] <= 5
        });
        assert!(
            top.checked > 50 && top.wrong.is_empty(),
            "{name}: the first shadow is on top: {:?}",
            &top.wrong[..top.wrong.len().min(5)]
        );
        let below = under_offset(base, &shaded, (device(4.0), device(4.0)), |p| {
            u16::from(p[0]) + u16::from(p[2]) >= 245 && p[1] <= 5
        });
        assert!(
            below.checked > 50 && below.wrong.is_empty(),
            "{name}: the second shadow fills what the first leaves: {:?}",
            &below.wrong[..below.wrong.len().min(5)]
        );
    }
    let blurred = Shadow::new([2.0, 2.0], 1.5, 0.0, RED);
    for ((name, base), (_, shaded)) in plain.iter().zip(shadowed(vec![blurred])) {
        let (dx, dy) = (device(2.0), device(2.0));
        let spread = shaded
            .enumerate_pixels()
            .filter(|(x, y, p)| {
                let (sx, sy) = (i64::from(*x) - dx, i64::from(*y) - dy);
                clear(base, *x, *y) && untouched(base, sx, sy) && p.0[0] > 40
            })
            .count();
        println!("{name}: blurred shadow spreads to {spread} pixels the glyphs never touch");
        assert!(
            spread > 50,
            "{name}: a blurred shadow spreads past the glyph outline"
        );
    }
}
