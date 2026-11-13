use sigil::prelude::*;
use sigil::scene::BlendMode;
use sigil_kittest::Harness;

use super::{
    BACKDROP, CELL, HEIGHT, SCALE, SOURCE, TOLERANCE, WIDTH, cpu_frame, model, probe, rgba,
};

fn blended_cell(mode: BlendMode) -> impl IntoNode {
    container()
        .layout(LayoutStyle::block().fixed_size(CELL, CELL))
        .background(rgba(BACKDROP, 255))
        .child(
            container()
                .layout(LayoutStyle::block().fixed_size(CELL, CELL))
                .background(rgba(SOURCE, 255))
                .blend(mode),
        )
}

fn styled_gallery() -> impl IntoNode {
    let mut row =
        container().layout(LayoutStyle::flex_row().fixed_size(WIDTH as f32, HEIGHT as f32));
    for mode in BlendMode::ALL {
        row = row.child(blended_cell(mode));
    }
    row
}

#[test]
fn a_node_with_mix_blend_mixes_with_its_parent_by_the_spec_formula() {
    let mut pixels = Harness::new(styled_gallery)
        .pixels(WIDTH, HEIGHT)
        .scale(SCALE);
    let mut surface = Harness::new(styled_gallery).gpu(WIDTH, HEIGHT).scale(SCALE);
    let scene = surface.host().emit((WIDTH as f32, HEIGHT as f32));
    let frames = [
        ("pixels", pixels.expect_image()),
        ("gpu", surface.expect_image()),
        ("cpu", cpu_frame(&scene)),
    ];
    let mut failures = Vec::new();
    for (name, frame) in &frames {
        for (column, mode) in BlendMode::ALL.into_iter().enumerate() {
            let expected = model::composite(mode, BACKDROP, SOURCE, 255);
            let found = probe(frame, column, 0);
            if found
                .iter()
                .zip(expected)
                .any(|(a, b)| a.abs_diff(b) > TOLERANCE)
            {
                failures.push(format!("{name} {mode:?}: {found:?} != {expected:?}"));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "mix_blend pixels off the spec formula: {failures:#?}"
    );
}
