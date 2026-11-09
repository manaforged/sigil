use sigil::prelude::*;

use super::{INK, SCALE, SURFACES, Shot, WHITE, distance, page, shoot};

const HALF_INK: u32 = 382;

fn card(width: f32, height: f32, border: i16) -> NodeBuilder {
    container()
        .layout(LayoutStyle::block().fixed_size(width, height))
        .bg(WHITE)
        .border_w(border)
        .border_color(INK)
}

pub(crate) fn dashed_card() -> impl IntoNode {
    page(120.0, 60.0, card(120.0, 60.0, 2).border_dashed())
}

fn rounded_dashed_card() -> impl IntoNode {
    page(
        120.0,
        60.0,
        card(120.0, 60.0, 2).rounded(Radius::Full).border_dashed(),
    )
}

fn dotted_card() -> impl IntoNode {
    page(200.0, 40.0, card(200.0, 40.0, 8).border_dotted())
}

fn inked(shot: &Shot, x: f32, y: f32) -> bool {
    distance(shot.at(x, y), WHITE) > HALF_INK
}

fn runs(row: &[bool]) -> Vec<(bool, usize)> {
    let mut out: Vec<(bool, usize)> = Vec::new();
    for &on in row {
        match out.last_mut() {
            Some((last, length)) if *last == on => *length += 1,
            _ => out.push((on, 1)),
        }
    }
    out
}

#[test]
fn a_dashed_border_has_dashes_three_widths_long_and_gaps_two_widths_long() {
    for surface in SURFACES {
        let shot = shoot(dashed_card, (120, 60), surface);
        let y = SCALE as u32;
        let row: Vec<bool> = (0..shot.image.width())
            .map(|x| {
                let pixel = shot.image.get_pixel(x, y);
                distance([pixel[0], pixel[1], pixel[2]], WHITE) > HALF_INK
            })
            .collect();
        let found = runs(&row);
        let inner = &found[2..found.len() - 2];
        println!("{surface:?}: top edge runs {inner:?}");
        assert!(
            inner.len() >= 10,
            "{surface:?}: the top edge breaks into dashes: {found:?}"
        );
        for (on, length) in inner {
            let want = if *on { 12 } else { 8 };
            assert!(
                length.abs_diff(want) <= 2,
                "{surface:?}: a {} run is {length} device px, want {want}±2 (6 and 4 px at 2x): {inner:?}",
                if *on { "dash" } else { "gap" }
            );
        }
    }
}

#[test]
fn a_dashed_border_follows_a_rounded_corner() {
    for surface in SURFACES {
        let shot = shoot(rounded_dashed_card, (120, 60), surface);
        let arc: Vec<bool> = (0..=180)
            .map(|step| {
                let angle = (step as f32 * 0.5).to_radians();
                inked(
                    &shot,
                    29.0f32.mul_add(angle.sin(), 90.0),
                    29.0f32.mul_add(-angle.cos(), 30.0),
                )
            })
            .collect();
        let found = runs(&arc);
        let dashes = found.iter().filter(|(on, _)| *on).count();
        println!("{surface:?}: corner arc runs {found:?}");
        assert!(
            dashes >= 3 && found.len() >= 6,
            "{surface:?}: the 45 px corner arc holds dashes and gaps: {found:?}"
        );
        assert!(
            !inked(&shot, 1.0, 1.0) && !inked(&shot, 119.0, 59.0),
            "{surface:?}: the square box corner stays empty outside the arc"
        );
    }
}

#[test]
fn a_dotted_border_draws_round_dots() {
    for surface in SURFACES {
        let shot = shoot(dotted_card, (200, 40), surface);
        assert!(
            inked(&shot, 20.0, 4.0),
            "{surface:?}: a dot centre at 20,4 is inked"
        );
        assert!(
            !inked(&shot, 28.0, 4.0),
            "{surface:?}: the gap between dots is empty"
        );
        for (dx, dy) in [(3.3, 3.3), (-3.3, 3.3), (3.3, -3.3), (-3.3, -3.3)] {
            let corner = (20.0 + dx, 4.0 + dy);
            assert!(
                !inked(&shot, corner.0, corner.1),
                "{surface:?}: {corner:?} lies inside a square dot but 4.7 px from the round dot's centre, so it is empty"
            );
        }
        for (dx, dy) in [(2.5, 0.0), (0.0, 2.5), (-2.5, 0.0), (0.0, -2.5)] {
            assert!(
                inked(&shot, 20.0 + dx, 4.0 + dy),
                "{surface:?}: 2.5 px from the centre lies inside the 4 px dot"
            );
        }
    }
}
