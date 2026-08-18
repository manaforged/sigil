//! DPI-scale golden axis — Wave 0 truth harness.
//!
//! Captures ONE text-bearing widget (a button with a real label) at four
//! DPI scales (1.0 / 1.25 / 1.5 / 2.0) through
//! [`render_widget_to_png_scaled`] using the REAL `sigil_app::DefaultTextMeasure`
//! (an `OwnedTextEngine`-backed measurer, not the deterministic stub), per
//! `docs/plans/2026-07-07-text-quality-and-gap-closure.md` Wave 0.
//!
//! IMPORTANT — these goldens capture TODAY'S baseline, not a corrected
//! pipeline. `crates/renderer/src/offscreen.rs:97,142` hardcode
//! `scale: 1.0` in the offscreen `WalkCtx` regardless of the
//! logical-viewport/physical-texture split expressed here, so every
//! capture at scale > 1.0 bakes the current stretched/soft output — the
//! same logical-space geometry rasterized into a larger texture with no
//! AA re-tuning for the higher pixel density. That IS the Wave-0 point:
//! these four PNGs are the visible "before" that Wave 1's DPI plumbing
//! (real `scale` threaded through `ScreenSubmission` + both offscreen
//! walkers) is expected to visibly change. Do not "fix" the renderer here
//! to make these look sharp — that is Wave 1's job, and Wave 1's gate is a
//! deliberate re-bake of this exact set with visual-review sign-off (see
//! the plan's Wave 1 section).
//!
//! Run with `SIGIL_REGEN_GOLDENS=1` to bake initially, then re-run to
//! verify green. Always with `--test-threads=1` — the wgpu device-init
//! race under parallel test threads (docs/followups.md) crashes this
//! suite otherwise.

#![allow(missing_docs)]
#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]
#![allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
#![allow(clippy::cast_sign_loss, clippy::doc_markdown)]
#![allow(clippy::print_stdout, clippy::too_many_lines)]

use arboard as _;
use criterion as _;
use sigil_actions as _;
use sigil_animation as _;
use sigil_bidi as _;
use sigil_events as _;
use sigil_kernel as _;
use sigil_profile as _;
use sigil_reactive as _;
use sigil_style as _;
use sigil_tasks as _;

use sigil::prelude::*;
use sigil_app::DefaultTextMeasure;

mod golden_support;

use golden_support::{
    InteractionMode, assert_goldens_differ, assert_png_matches_golden, golden_path,
    render_widget_to_png_scaled,
};

const DELTA_PCT: f32 = 0.5;
const LOGICAL_SIZE: [u32; 2] = [160, 56];
const HIT_POINT: (f32, f32) = (74.0, 22.0);

/// The four scales in the Wave 0 axis, paired with their golden stems.
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

/// Ghost-pass guard: the four scale captures must not collapse to
/// byte-identical PNGs. Because physical texture dimensions differ by
/// construction (160x56 logical * scale), this also catches the coarser
/// failure of a capture silently reusing another scale's texture size —
/// not just a same-size-different-content regression.
#[test]
fn button_label_dpi_scales_are_pairwise_distinct() {
    for (i, (stem_a, _)) in SCALE_STEMS.iter().enumerate() {
        for (stem_b, _) in &SCALE_STEMS[i + 1..] {
            assert_goldens_differ(stem_a, stem_b);
        }
    }
}
