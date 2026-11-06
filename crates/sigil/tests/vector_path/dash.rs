use sigil::prelude::*;
use sigil::scene::SceneBuilder;
use sigil::geometry::Rect;
use sigil::widgets::{canvas, path};
use sigil_kittest::Harness;

use super::{INK, coverage, frame_of};

fn dashed() -> impl IntoNode {
    canvas(|scene: &mut SceneBuilder, rect: Rect| {
        path(100, 20)
            .move_to(10.0, 10.0)
            .line_to(90.0, 10.0)
            .stroke(INK, 2.0)
            .dash(&[4.0, 4.0])
            .draw(scene, rect, &Theme::light());
    })
    .width(100.0)
    .height(20.0)
}

fn runs(lit: &[bool]) -> Vec<(bool, usize)> {
    lit.iter().fold(Vec::new(), |mut runs, &on| {
        match runs.last_mut() {
            Some((state, len)) if *state == on => *len += 1,
            _ => runs.push((on, 1)),
        }
        runs
    })
}

#[test]
fn a_four_four_dash_alternates_four_px_runs_and_gaps() {
    let mut ui = Harness::new(dashed).pixels(100, 20).scale(1.0);
    let frame = frame_of(&mut ui);
    let lit: Vec<bool> = (10..90).map(|x| coverage(&frame, x, 10) > 0.5).collect();
    let found = runs(&lit);
    println!("dash runs (lit, px): {found:?}");
    assert_eq!(found.len(), 20, "80 px of a [4, 4] dash: 10 runs, 10 gaps");
    for (on, len) in &found {
        assert!(
            (3..=5).contains(len),
            "a {} run is {len} px; a [4, 4] dash has 4 px ±1",
            if *on { "lit" } else { "gap" }
        );
    }
}
