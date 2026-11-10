#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]
#![allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
#![allow(clippy::cast_sign_loss, clippy::doc_markdown)]
#![allow(clippy::print_stdout, clippy::too_many_lines)]

use sigil::app::DefaultTextMeasure;
use sigil::prelude::*;

use sigil_kittest::{Golden, Harness, InteractionMode};

const DELTA_PCT: f32 = 0.5;
const LOGICAL_SIZE: [u32; 2] = [160, 56];
const HIT_POINT: (f32, f32) = (74.0, 22.0);

const SCALE_STEMS: [(&str, f32); 4] = [
    ("dpi/button_label_s100", 1.0),
    ("dpi/button_label_s125", 1.25),
    ("dpi/button_label_s150", 1.5),
    ("dpi/button_label_s200", 2.0),
];

fn build_button() -> impl IntoNode {
    button("Save")
}

fn capture_at_scale(stem: &str, scale: f32) {
    let img = Harness::render(build_button)
        .measure(|| Box::new(DefaultTextMeasure::new()))
        .mode(InteractionMode::Default)
        .size(LOGICAL_SIZE)
        .scale(scale)
        .hit(HIT_POINT)
        .image()
        .expect("sigil-kittest");
    let Some(img) = img else {
        println!("no GPU adapter — skipping {stem}");
        return;
    };
    Golden::new(stem).delta(DELTA_PCT).assert_image(&img);
}

#[test]
fn button_label_dpi_scale_100() {
    capture_at_scale(SCALE_STEMS[0].0, SCALE_STEMS[0].1);
}

#[test]
fn button_label_dpi_scale_125() {
    capture_at_scale(SCALE_STEMS[1].0, SCALE_STEMS[1].1);
}

#[test]
fn button_label_dpi_scale_150() {
    capture_at_scale(SCALE_STEMS[2].0, SCALE_STEMS[2].1);
}

#[test]
fn button_label_dpi_scale_200() {
    capture_at_scale(SCALE_STEMS[3].0, SCALE_STEMS[3].1);
}

#[test]
fn button_label_dpi_scales_are_pairwise_distinct() {
    for (i, (stem_a, _)) in SCALE_STEMS.iter().enumerate() {
        for (stem_b, _) in &SCALE_STEMS[i + 1..] {
            Golden::new(stem_a).assert_differs(&Golden::new(stem_b));
        }
    }
}
