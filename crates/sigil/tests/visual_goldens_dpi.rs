#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(missing_docs)]
#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]
#![allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
#![allow(clippy::cast_sign_loss, clippy::doc_markdown)]
#![allow(clippy::print_stdout, clippy::too_many_lines)]

use sigil::app::DefaultTextMeasure;
use sigil::prelude::*;

mod golden_support;

use golden_support::{
    InteractionMode, assert_goldens_differ, assert_png_matches_golden, golden_path,
    render_widget_to_png_scaled,
};

const DELTA_PCT: f32 = 0.5;
const LOGICAL_SIZE: [u32; 2] = [160, 56];
const HIT_POINT: (f32, f32) = (74.0, 22.0);

const SCALE_STEMS: [(&str, f32); 4] = [
    ("dpi/button_label_s100", 1.0),
    ("dpi/button_label_s125", 1.25),
    ("dpi/button_label_s150", 1.5),
    ("dpi/button_label_s200", 2.0),
];

fn build_button(cx: &mut Cx) -> NodeId {
    cx.add(button("Save"))
}

fn capture_at_scale(stem: &str, scale: f32) {
    let img = render_widget_to_png_scaled(
        build_button,
        InteractionMode::Default,
        LOGICAL_SIZE,
        scale,
        HIT_POINT,
        Box::new(DefaultTextMeasure::new()),
    );
    let Some(img) = img else {
        println!("no GPU adapter — skipping {stem}");
        return;
    };
    assert_png_matches_golden(&img, &golden_path(stem), DELTA_PCT);
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
            assert_goldens_differ(stem_a, stem_b);
        }
    }
}
